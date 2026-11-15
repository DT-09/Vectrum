use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::{BTreeMap, BTreeSet, VecDeque}, fs, path::Path};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all="snake_case")]
pub enum Compatibility { Compatible, Breaking, ReviewRequired }
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all="snake_case")]
pub enum ChangeKind { Added, Removed, Modified }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Change {
    pub path:String, pub kind:ChangeKind, pub compatibility:Compatibility,
    pub summary:String, pub evidence:String,
    pub before:Option<Value>, pub after:Option<Value>
}
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Contract {
    pub name:String, pub format:String,
    pub entities:BTreeMap<String,Value>
}
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DependencyGraph {
    #[serde(default)] pub nodes:Vec<String>,
    #[serde(default)] pub edges:Vec<Edge>
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge { pub from:String, pub to:String }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Impact { pub changed_contract:String, pub affected_consumers:Vec<String>, pub evidence:String }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Report {
    pub schema_version:String, pub engine:String, pub baseline:String, pub candidate:String,
    pub compatibility:Compatibility, pub summary:String, pub changes:Vec<Change>,
    pub impacts:Vec<Impact>, pub limitations:Vec<String>
}

pub fn load(path:&Path)->anyhow::Result<Contract>{
    let raw=fs::read_to_string(path)?;
    let value:Value=serde_json::from_str(&raw)?;
    Ok(normalize(&value,path.file_name().and_then(|s|s.to_str()).unwrap_or("contract")))
}
pub fn normalize(v:&Value,fallback:&str)->Contract{
    let format=if v.get("openapi").is_some()||v.get("swagger").is_some(){"openapi"}
        else if v.get("$schema").is_some()||v.get("properties").is_some(){"json_schema"}else{"json"};
    let mut entities=BTreeMap::new();
    if format=="openapi" {
        if let Some(paths)=v.get("paths").and_then(Value::as_object){
            for (path,item) in paths { if let Some(methods)=item.as_object(){
                for (method,op) in methods {
                    if ["get","put","post","delete","patch","head","options","trace"].contains(&method.as_str()){
                        entities.insert(format!("operation:{}:{}",method.to_uppercase(),path),op.clone());
                    }
                }
            }}
        }
        if let Some(schemas)=v.pointer("/components/schemas").and_then(Value::as_object){
            for (name,schema) in schemas { flatten(schema,&format!("schema:{}",name),&mut entities); }
        }
    } else { flatten(v,"root",&mut entities); }
    if entities.is_empty(){entities.insert("document".into(),v.clone());}
    let name=v.pointer("/info/title").and_then(Value::as_str).or_else(||v.get("title").and_then(Value::as_str)).unwrap_or(fallback).to_string();
    Contract{name,format:format.into(),entities}
}
fn flatten(v:&Value,prefix:&str,out:&mut BTreeMap<String,Value>){
    out.insert(prefix.into(),v.clone());
    if let Some(props)=v.get("properties").and_then(Value::as_object){
        for (key,value) in props { flatten(value,&format!("{}.properties.{}",prefix,key),out); }
    }
    if let Some(items)=v.get("items"){flatten(items,&format!("{}.items",prefix),out);}
}
pub fn analyze(old:&Contract,new:&Contract,graph:&DependencyGraph)->Report{
    let keys:BTreeSet<String>=old.entities.keys().chain(new.entities.keys()).cloned().collect();
    let mut changes=Vec::new();
    for path in keys {
        let before=old.entities.get(&path); let after=new.entities.get(&path);
        if before==after {continue;}
        let (kind,compatibility,summary,evidence)=match(before,after){
            (None,Some(_))=>(ChangeKind::Added,Compatibility::Compatible,"Contract element added".into(),"Element exists only in candidate.".into()),
            (Some(_),None)=>(ChangeKind::Removed,Compatibility::Breaking,"Contract element removed".into(),"Element exists in baseline and is absent from candidate.".into()),
            (Some(a),Some(b))=>classify(a,b),
            _=>unreachable!()
        };
        changes.push(Change{path,kind,compatibility,summary,evidence,before:before.cloned(),after:after.cloned()});
    }
    let compatibility=if changes.iter().any(|c|c.compatibility==Compatibility::Breaking){Compatibility::Breaking}
        else if changes.iter().any(|c|c.compatibility==Compatibility::ReviewRequired){Compatibility::ReviewRequired}
        else{Compatibility::Compatible};
    let impacts=changes.iter().map(|c|Impact{
        changed_contract:c.path.clone(),affected_consumers:affected(graph,&c.path),
        evidence:"Reverse traversal of caller-supplied dependency edges; graph completeness is not verified.".into()
    }).collect::<Vec<_>>();
    let breaking=changes.iter().filter(|c|c.compatibility==Compatibility::Breaking).count();
    let review=changes.iter().filter(|c|c.compatibility==Compatibility::ReviewRequired).count();
    let compatible=changes.len()-breaking-review;
    Report{schema_version:"1.0".into(),engine:env!("CARGO_PKG_VERSION").into(),baseline:old.name.clone(),candidate:new.name.clone(),compatibility,
        summary:format!("{} change(s): {} breaking, {} requiring review, {} compatible.",changes.len(),breaking,review,compatible),
        changes,impacts,limitations:vec![
            "Supported input: JSON, JSON Schema-like JSON, and OpenAPI represented as JSON.".into(),
            "Compatibility classification is structural and conservative; it does not prove runtime substitutability.".into(),
            "Dependency impact depends entirely on the graph supplied by the caller.".into(),
            "YAML, Protobuf, SQL DDL, external reference resolution, and runtime simulation are not implemented.".into()
        ]}
}
fn classify(a:&Value,b:&Value)->(ChangeKind,Compatibility,String,String){
    if a.get("type")!=b.get("type")&&(a.get("type").is_some()||b.get("type").is_some()){
        return(ChangeKind::Modified,Compatibility::Breaking,"Declared type changed".into(),format!("Type changed from {:?} to {:?}.",a.get("type"),b.get("type")));
    }
    if let (Some(x),Some(y))=(a.get("enum").and_then(Value::as_array),b.get("enum").and_then(Value::as_array)){
        if x.iter().any(|v|!y.contains(v)){return(ChangeKind::Modified,Compatibility::Breaking,"Allowed values narrowed".into(),"Candidate removes one or more previously allowed enum values.".into());}
    }
    let old_req:BTreeSet<&Value>=a.get("required").and_then(Value::as_array).into_iter().flatten().collect();
    let new_req:BTreeSet<&Value>=b.get("required").and_then(Value::as_array).into_iter().flatten().collect();
    if new_req.difference(&old_req).next().is_some(){return(ChangeKind::Modified,Compatibility::Breaking,"New required field introduced".into(),"Candidate requires a field not required by baseline.".into());}
    if b.get("deprecated")==Some(&Value::Bool(true))&&a.get("deprecated")!=b.get("deprecated"){
        return(ChangeKind::Modified,Compatibility::ReviewRequired,"Element deprecated".into(),"Deprecation needs a consumer migration policy.".into());
    }
    (ChangeKind::Modified,Compatibility::ReviewRequired,"Contract definition changed".into(),"No deterministic compatibility rule matched; manual review is required.".into())
}
pub fn affected(graph:&DependencyGraph,changed:&str)->Vec<String>{
    let mut reverse:BTreeMap<&str,Vec<&str>>=BTreeMap::new();
    for edge in &graph.edges{reverse.entry(&edge.to).or_default().push(&edge.from);}
    let mut seen=BTreeSet::new();let mut queue=VecDeque::from([changed.to_string()]);
    while let Some(node)=queue.pop_front(){if let Some(consumers)=reverse.get(node.as_str()){
        for consumer in consumers{if seen.insert((*consumer).to_string()){queue.push_back((*consumer).to_string());}}
    }}
    seen.into_iter().collect()
}
#[cfg(test)]
mod tests{
 use super::*; use serde_json::json;
 fn c(v:Value)->Contract{let mut entities=BTreeMap::new();entities.insert("root".into(),v);Contract{name:"test".into(),format:"json".into(),entities}}
 #[test]fn type_change_is_breaking(){let r=analyze(&c(json!({"type":"string"})),&c(json!({"type":"integer"})),&DependencyGraph::default());assert_eq!(r.compatibility,Compatibility::Breaking);}
 #[test]fn identical_contract_is_compatible(){let x=c(json!({"type":"string"}));assert_eq!(analyze(&x,&x,&DependencyGraph::default()).changes.len(),0);}
 #[test]fn removed_contract_is_breaking(){let a=c(json!({"type":"string"}));let b=Contract{name:"b".into(),format:"json".into(),entities:BTreeMap::new()};assert_eq!(analyze(&a,&b,&DependencyGraph::default()).compatibility,Compatibility::Breaking);}
 #[test]fn traces_transitive_consumers(){let g=DependencyGraph{nodes:vec![],edges:vec![Edge{from:"checkout".into(),to:"pricing".into()},Edge{from:"web".into(),to:"checkout".into()}]};assert_eq!(affected(&g,"pricing"),vec!["checkout","web"]);}
}
