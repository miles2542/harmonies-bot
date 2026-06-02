# Harmonies BGA Bot: Recommended Architecture

## 1. Core Architecture Decompositions
* **Single-Player Optimization:** Board placement search. 3 tokens placed per turn. Large combinatorics.
* **Token Draft Layer:** 5 choices from central board. Deny opponent draft if high-value.
* **Unified Model:** Single evaluator handles Side A & Side B. Flag `is_side_b` enables side-specific features (Rivers Side A / Islands Side B). Zero-out irrelevant features.
* **Nature's Spirit Integration:** In scope from setup. Hand limit (4 cards) shares space with Spirit. Spirit scoring targets (e.g., Mountain height counts, Yellow groups) mapped to evaluation features.

---

## 2. Bot Decision Algorithm & Time Management
* **Search Strategy:** Stochastic Beam Search.
  * **Width ($K$):** 50-100 (dynamic based on time budget).
  * **Depth ($N$):** 3-4 turns.
  * **Bag Uncertainty ($M$):** 10-15 random token samples. Average leaf scores.
* **Pondering & Memoization (Opponent Turn):**
  * Board state static on opponent turn.
  * Web Worker runs search branches in background during opponent turn.
  * Results cached in Transposition Table (hash map of evaluated boards).
  * Our turn search hits cache $\rightarrow$ instant result or permits deeper search ($N=5-6$).
* **Real-time Worker Execution:**
  * Browser extension monitors page state.
  * Auto-detects our turn $\rightarrow$ invokes WASM search engine in background Web Worker.
  * Streams current best move sequence to UI.
  * "Get Move Now" manual override button aborts search, outputs best cached sequence instantly.

---

## 3. Parametric Evaluation Function ($W \cdot F$)
Evaluation score is weighted sum of features $F$.

### Landscape Features:
* **Immediate Score:** Points from complete structures.
* **Trees:** Browns adjacent to Greens, number of Tall Trees vs Bushes.
* **Mountains:** Size of mountain clusters (isolated = 0 pts).
* **Fields:** Size of Yellow groups (optimal size $\ge 2$).
* **Buildings:** Adjacent color diversity of Red stacks.
* **Water (Side A):** Rivers length, path endpoints extendability.
* **Water (Side B):** Island count, enclosure potential.

### Card & Spirit Features:
* **Card Progress:** Fraction of animal pattern matching.
* **Draft Utility:** Expected turns to complete hand cards.
* **Spirit Progress:** Completion status, spirit-specific landscape scoring.

### Strategy / Constraints / Denial:
* **Waste Penalty:** Isolated tokens with no growth path.
* **Space Congestion:** Board fill rate, empty hex count.
* **Hand Congestion Penalty:** Penalty for high card count in hand (reduced draft flexibility, max 4).
* **Deny Token Value:** Opponent gain from drafted token.
* **Deny Card Value:** Opponent gain from drafted card.
* **Deny Spirit Value:** Opponent progress toward Spirit card objective.
* **Game End Rush Weight:** Rush game end (deplete empty spaces) if score diff $> 0$, delay if $< 0$.

---

## 4. Parameter Tuning (CMA-ES)
* **Optimization Method:** CMA-ES (Covariance Matrix Adaptation Evolution Strategy).
* **Execution:** Python runner orchestrates native Rust engine via `PyO3`.
* **Hardware Utilization:** 12600K CPU (16 threads). Parallel game simulations via Rust `rayon`. No GPU training needed.
* **Fitness Function:** 
  * 2-Player: $score_{self} - score_{opponent}$.
  * 3/4-Player: $score_{self} - \text{mean}(score_{opponents})$.
* **Speed-up Strategy:** Use shallow Beam Search ($K=30$, $N=3$) for self-play data generation.

---

## 5. Technology Stack & Data Flow
```mermaid
graph TD
    subgraph Browser Client (Serverless Portable)
        A[BGA Gameplay Page] -->|Read DOM & gameui.gamedatas| B[Firefox Extension JS]
        B -->|Start Search| Worker[Web Worker WASM Engine]
        Worker -->|Transposition Cache / Ponder| Worker
        Worker -->|Stream Move Sequence| B
        B -->|Render UI Overlay| A
    end
    subgraph Local Training Stack
        D[Python Tuner] -->|CMA-ES weights| PyO3[Native Rust Wrapper PyO3]
        PyO3 -->|Rayon Parallel Self-Play| D
        PyO3 -->|Export Tuned Weights JSON| Worker
    end
```

* **Frontend:** Firefox Extension (Manifest V2). Reads `window.gameui.gamedatas`.
* **Gameplay Search Engine:** WebAssembly (WASM) compiled from Rust. Runs inside Extension Web Worker. Portable, client-only.
* **Tuning Pipeline:** Python script running CMA-ES + native Rust binary via `PyO3` for fast multithreaded simulations.

