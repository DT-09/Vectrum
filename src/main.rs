use anyhow::Context;
use clap::{Parser,Subcommand};
use std::{fs,path::PathBuf};
use vectrum::{analyze,load,DependencyGraph,Compatibility};

#[derive(Parser)]
#[command(name="vectrum",version,about="Deterministic compatibility intelligence for software contracts")]
struct Cli{#[command(subcommand)] command:Command}
#[derive(Subcommand)]
enum Command{
 Compare{
  #[arg(long)] baseline:PathBuf,#[arg(long)] candidate:PathBuf,
  #[arg(long)] graph:Option<PathBuf>,#[arg(long)] output:Option<PathBuf>,
  #[arg(long,default_value_t=false)] fail_on_breaking:bool
 },
 Info
}
fn main()->anyhow::Result<()>{
 match Cli::parse().command{
  Command::Info=>println!("VECTRUM {}\nInputs: JSON, JSON Schema-like JSON, OpenAPI JSON\nReport schema: 1.0",env!("CARGO_PKG_VERSION")),
  Command::Compare{baseline,candidate,graph,output,fail_on_breaking}=>{
   let old=load(&baseline).with_context(||format!("loading {}",baseline.display()))?;
   let new=load(&candidate).with_context(||format!("loading {}",candidate.display()))?;
   let graph=if let Some(path)=graph{serde_json::from_str::<DependencyGraph>(&fs::read_to_string(&path).with_context(||format!("reading {}",path.display()))?).context("invalid dependency graph")?}else{DependencyGraph::default()};
   let report=analyze(&old,&new,&graph);
   let rendered=serde_json::to_string_pretty(&report)?;
   if let Some(path)=output{if let Some(parent)=path.parent(){if !parent.as_os_str().is_empty(){fs::create_dir_all(parent)?;}}fs::write(path,rendered)?;}else{println!("{}",rendered);}
   if fail_on_breaking&&report.compatibility==Compatibility::Breaking{std::process::exit(2);}
  }
 }
 Ok(())
}
