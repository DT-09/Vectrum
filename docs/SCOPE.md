# Product scope

VECTRUM is a deterministic structural compatibility analyzer. It accepts JSON documents, JSON Schema-like JSON, and OpenAPI represented as JSON. It normalizes contract elements, compares baseline and candidate snapshots, classifies changes, optionally traverses a caller-supplied dependency graph, and emits JSON evidence.

## Compatibility policy

- Added elements: compatible by default.
- Removed elements: breaking.
- Declared type changes: breaking.
- Enum narrowing: breaking.
- New required fields: breaking.
- Newly deprecated elements: review required.
- Other modifications: review required.

A compatible result means no implemented breaking rule was triggered. It is not proof of runtime substitutability. A review-required result is intentional: the engine does not invent certainty.

## Dependency graph

Edges point from consumer to dependency. VECTRUM traverses the reverse graph to find direct and transitive consumers of a changed contract. The graph is caller-supplied; automatic source discovery is outside current scope.

## Not implemented

YAML parsing, Protobuf, SQL DDL, complete JSON Schema reference resolution, source-code dependency discovery, runtime simulation, and automatic migration generation are not claimed by this release.
