# Architecture Overview (Draft)

> **Axiom** — "Cada dobra de proteína é um contrato rotacional computável, codificado em unidades de tempo (ms), energia (ΔS) e informação (spans)."

O workspace traduz esse axioma em uma pilha de camadas que convertem sequências físicas (`FASTA`, `PDB`) em contratos `.lll` auditáveis e reversíveis.

```
workspace/
├── core/         # Runtime enzimático (parser, solver, enforcement, rollback)
├── time/         # Relógio rotacional, trajetórias e métricas de entropia
├── molecule/     # Aminoácidos, cadeia peptídica, restrições e energia
├── interface/    # CLI/REST, loaders, integração LogLine
├── sim/          # Orquestração de simulações, métricas e visualização
└── app/          # Binário de entrada + presets e contratos de exemplo
```

## Camadas Computáveis

1. **Input & Parsing** (`interface/input_loader.rs`, `core/folding_parser.rs`)
   - Converte FASTA/PDB em `PeptideChain` com posições espaciais iniciais.
   - Compila `.lll` / `.fold` em `ContractInstruction` (`rotate`, `clash_check`, `ghost`, etc.).

2. **Estado & Tempo** (`molecule/`, `time/`)
   - `PeptideChain` mantém ângulos φ/ψ + coordenadas `[x,y,z]`.
   - `RotationClock` e `MsRuntimeExecutor` definem o passo rotacional (1 rotação = 1 ms por padrão).
   - `Trajectory` agrega ΔS / Δi e suporta rollback (`pop_last`).

3. **Execução & Simulação** (`core/rotation_solver.rs`, `core/protein_state.rs`)
   - `RotationSolver` aplica micro rotações com oscilador e gera `SpanRecord`.
   - `ProteinState` atualiza ângulos + projeção espacial, preservando instantâneos para rollback.
   - `ghost_mode` armazena spans exploratórios sem comprometê-los com a timeline principal.

4. **Validação & Enforcement** (`core/folding_ruleset.rs`, `core/validation.rs`)
   - Limites de rotação, orçamentos de entropia/informação e distância mínima molecular.
   - `validate_structure` detecta colisões estruturais (clash) usando distâncias Euclidianas.
   - `ClashCheck` executável aciona verificação explícita durante o contrato.

5. **Registro & Proveniência** (`interface/logline_integration.rs`, `sim/folding_log.rs`)
   - `LogLineWriter` separa spans aplicados, spans fantasma e violações.
   - `FoldingMetrics` resume ΔS/Δi para spans reais e fantasmas.
   - Diamonds (trajetórias estáveis) ficam prontos para persistência externa.

6. **Interface & Controle** (`interface/command_shell.rs`, `app/main.rs`)
   - CLI executa presets ou contratos fornecidos, consolida logs e métricas.
   - Simulador (`sim/folding_simulator.rs`) roda pipelines end-to-end para benchmarks.

## Pipeline de Execução

1. Recebe sequência (`InputLoader::load_fasta`) + contrato `.lll`.
2. Builder compõe `FoldingEngine` com regras (`Ruleset`) e relógio (`RotationClock`).
3. Para cada instrução:
   - `RotationSolver` calcula rotação + span.
   - Budgets de ΔS/Δi são avaliados antes de materializar o span.
   - Estado molecular é rotacionado; `check_structure` detecta clashes e reverte se preciso.
   - Spans válidos entram no `Trajectory`; spans fantasmas permanecem segregados.
4. `ClashCheck`, `Commit`, `Rollback` e `GhostMode` modulam a linha do tempo computável.
5. Relatórios consolidam rotações aplicadas/ghost, violações e energia final.

## Próximos Marcos
- Refino biofísico do `EnergyModel` com potenciais reais (Lennard-Jones, Coulomb).
- Enriquecer `PhysicsSimulator` e enforcement de ligações/ânulos + temperatura.
- Visualização 3D (WebGPU/WASM) acoplada ao `TrajectoryVisualizer`.
- Persistência distribuída de spans/diamonds (IPFS ou ledger LogLine Vault).
