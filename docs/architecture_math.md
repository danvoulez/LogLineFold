# LogLine Folding Engine v1 — MN-Esquema Arquitetural com Matemática Reforçada

> “Cada dobra de proteína é um contrato rotacional computável, codificado em unidades de tempo (ms), energia (ΔG, ΔS) e informação (spans).”

## Fundamentos Matemáticos

- **Rotações no Espaço 3D**: cada rotação é representada por uma matriz de rotação \(R(\theta, \vec{u})\), onde \(\theta\) é o ângulo de rotação e \(\vec{u}\) o eixo unitário. A nova posição atômica resulta de
  \[
  \vec{r}_{\text{new}} = R \cdot \vec{r}_{\text{old}}
  \]
- **Energia Livre de Gibbs**: minimizar \(G = E - T \Delta S\), com \(E\) energia total, \(T\) temperatura e \(\Delta S\) variação de entropia.
- **Energia de Campo Molecular**:
  \[
  E = E_{\text{bond}} + E_{\text{angle}} + E_{\text{dihedral}} + E_{\text{vdw}} + E_{\text{elec}} + E_{\text{hb}}
  \]
  com termos específicos:
  - \(E_{\text{bond}} = \sum k_b (r - r_0)^2\)
  - \(E_{\text{angle}} = \sum k_\theta (\theta - \theta_0)^2\)
  - \(E_{\text{dihedral}} = \sum k_\phi [1 + \cos(n\phi - \delta)]\)
  - \(E_{\text{vdw}} = \sum_{i<j} \left( \frac{A_{ij}}{r_{ij}^{12}} - \frac{B_{ij}}{r_{ij}^{6}} \right)\)
  - \(E_{\text{elec}} = \sum_{i<j} \frac{q_i q_j}{4\pi \epsilon_0 r_{ij}}\)
  - \(E_{\text{hb}} = \sum \left( \frac{C}{r^{12}} - \frac{D}{r^{10}} \right)\)
- **Entropia Computável**: via Shannon \(S = -k_B \sum_i p_i \ln p_i\).
- **Critério de Metropolis**: aceitação probabilística \(P = \min(1, e^{-\Delta E / k_B T})\); o motor suporta cronogramas de temperatura (annealing) para modular a aceitação ao longo do tempo.

## Arquitetura por Camadas

### 1. Camada de Input & Parsing (`/input`)
- `ProteinParser.rs`: mapeia FASTA/PDB em grafos moleculares (nós = resíduos, \(\vec{r}_i\)).
- `SpanInitiator.rs`: inicializa spans com \((\vec{r}_i, \theta_i, E_i, S_i)\).
- `LLL_Compiler.rs`: compila regras de folding em contratos `.lll` com restrições matemáticas (ex.: \(\theta_{\text{max}}, E_{\text{threshold}}\)).

### 2. Camada de Estado & Tempo (`/state`)
- `ProteinState.rs`: mantém tensor de posições \(\vec{r}_i\) e matrizes \(R_i\), atualizando \(\vec{r}_i(t + \Delta t) = \vec{r}_i(t) + \Delta \vec{r}_i\).
- `RotationClock.rs`: define passo temporal \(\Delta t\); tempo computável \(t_c = n \cdot \Delta t\).
- `TrajectoryLedger.rs`: registra a série temporal \((t, \vec{r}, E, S)\).

### 3. Camada de Execução & Simulação (`/engine`)
- `RotationEngine.rs`: propõe \(\Delta\theta\) via gradiente ou amostragem; aplica rotações pela matriz \(R(\Delta\theta, \vec{u})\).
- `PhysicsSimulator.rs`: computa \(E_{\text{total}}\) com listas de vizinhança e cutoffs.
- `MicroOscillator.rs`: modela onda informacional proporcional a \(|\Delta E|\).

### 4. Camada de Validação & Enforcement (`/validation`)
- `RuleEnforcer.rs`: garante restrições \(r_{ij} > r_{\text{min}}\) e \(\theta_{\text{min}} \leq \theta \leq \theta_{\text{max}}\).
- `ContractValidator.rs`: aplica contratos `.lll` com inequações (\(\Delta E < 0\), \(\Delta S > 0\)).
- `RollbackManager.rs`: reverte estados se \(E_{\text{total}} > E_{\text{max}}\).

### 5. Camada de Registro & Logging (`/log`)
- `SpanRecorder.rs`: grava spans com \(\Delta\theta, \Delta E, \Delta S\) e assinatura computacional.
- `LogLineIntegrator.rs`: formata spans para o ecossistema LogLine, adicionando métricas.
- `DiamondGenerator.rs`: identifica conformações onde \(G < G_{\text{threshold}}\) (\(G = E - T \Delta S\)).

### 6. Camada de Interface & Controle (`/interface`)
- `CLI_Shell.rs`: aceita parâmetros (T, \(\Delta t\), constantes de força).
- `REST_API.rs`: endpoints para envio de jobs e consulta de resultados.
- `LiveVisualizer.rs`: exibe trajetória 3D e curvas de energia.

## Fluxo de Dados Computável

1. **Input**: sequência → \(\vec{r}_i\) iniciais pelo ProteinParser.
2. **Inicialização**: spans iniciais com \(E_0, S_0\).
3. **Loop de Simulação** para cada \(\Delta t\):
   - `RotationEngine` propõe \(\Delta\theta\) e \(\vec{r}_{\text{new}}\).
   - `PhysicsSimulator` calcula \(E_{\text{new}}\) e \(\Delta E\).
   - `RuleEnforcer` valida; Metropolis aceita/rejeita.
   - Estado atualizado e span registrado via `SpanRecorder`.
4. **Convergência**: se \(|\Delta E| < \epsilon\) por N passos, `DiamondGenerator` registra o Diamond.
5. **Output**: log completo com dados matemáticos.

## Métricas de Saída

- Energia total final \(E_{\text{total}}\).
- Variação de entropia \(\Delta S\).
- Trabalho computacional \(\sum |\Delta E| \cdot \Delta t\).
- Curva \(E\) vs \(t\).
- Eficiência informacional \(\eta = \frac{\Delta S}{\text{traj}}\) (traj = total de rotações).

## Modos de Operação

- **Execução**: registro completo de spans com \(E\) e \(S\).
- **Simulação/Ghost**: sandbox com \(\Delta E\) simplificado.
- **Replay**: reprocessa spans registrados, recalculando \(E\) e \(S\) quando necessário.

Este esquema integra mecânica molecular, teoria da informação e otimização em um runtime computável audível pelo LogLine Folding Engine.
