Here is the final, verified, and bot-ready version of the rulebook. This version incorporates all corrections regarding token counts, stacking logic (specifically for trees), scoring values, and turn structure to align perfectly with the official Libellud rules and Board Game Arena (BGA) implementation.

---

# Harmonies: Bot-Ready Rulebook (Verified)

## 1. Overview & Objective
Players build vibrant, 3D ecosystems by arranging colored landscape tokens on a personal board. These landscapes create habitats to settle animals. Victory points (VP) are scored from landscape patterns and settled animal cards. The player with the highest VP total wins.

## 2. Components
*   **Central Board:** Dual-sided (adjusted for player count). Contains 5 spaces for token groups.
*   **Landscape Tokens:** 120 total tokens in a bag.
    *   *Correct Distribution:* 23 Grey, 23 Blue, 21 Brown, 19 Green, 19 Yellow, 15 Red.
*   **Animal Cards:** 5 face-up in the central display at all times.
*   **Animal Cubes:** Finite supply (66 total). Used to mark settled animals.
*   **Player Boards:** Dual-sided. Side A (Rivers), Side B (Islands). All players must use the same side.
*   **Nature's Spirit Cards (Optional):** 10 total. Used for the advanced variant.
*   **Nature's Spirit Cubes (Optional):** 4 total (one per player).

## 3. Gameplay Loop
Played clockwise. Each turn, you must perform the Mandatory Action, and you may perform the Optional Actions. These actions can be performed in any order, and optional actions can be interleaved between placing individual tokens from your mandatory action.

### Nature's Spirit Variant Setup
If playing with the Nature's Spirit variant, perform the following setup:
1.  **Deal:** Shuffle the 10 Nature's Spirit cards and deal 2 cards facedown to each player.
2.  **Select:** On their first turn, each player looks at their 2 cards, chooses 1 to place face-up above their Personal board, and returns the other to the game box.
3.  **Prepare:** Place 1 Nature's Spirit cube onto the chosen card. This card counts towards the player's 4-card hand limit until it is completed.

### Mandatory Action: Take & Place Tokens
Must be performed exactly once per turn.
1.  **Take:** Choose 1 of the 5 spaces on the Central Board. Take **all 3 tokens** from that space.
2.  **Place:** Place all 3 tokens onto your Personal board.
    *   You cannot keep tokens in hand; they must be placed on the board during your turn.
    *   You may place them on different spaces or stack them (following Stacking Rules below).
    *   Optional actions (Drafting or Settling) can be performed before, during, or after token placement (e.g., between placing the 1st and 2nd tokens).

#### Stacking Rules (Logic Constraints)
A token can only be placed if specific conditions are met.
*   **Max Height:** 3 tokens per space.
*   **Occupied Spaces:** If a space has an Animal Cube or a Nature's Spirit cube, no tokens can be placed or stacked on it.
*   **No Under-stacking:** You cannot place a token underneath previously placed tokens.
*   **Specific Token Rules:**
    *   **Grey (Mountain):** Can be placed on **Empty** or on top of **Grey** (up to max height 3).
    *   **Brown (Trunk):** Can be placed on **Empty** or on top of **Brown** (up to max height 2 of Brown tokens).
    *   **Green (Tree/Foliage):** Can be placed on **Empty** (scoring as a Bush) or on top of **Brown** (max height 2 of Brown, with Green always on top). No token can be placed on top of a Green token.
    *   **Red (Building):** MUST be placed on top of **Brown**, **Grey**, or **Red** (cannot be placed on an empty space; max height 3).
    *   **Yellow (Field):** Can ONLY be placed on **Empty** (Max height 1).
    *   **Blue (Water):** Can ONLY be placed on **Empty** (Max height 1).

### Optional Actions
You may perform the following actions in any order during your turn (or none):

#### Option A: Draft Animal Card (Max Once Per Turn)
*   **Draft:** Take 1 face-up Animal Card from the display.
*   **Place:** Place it face-up in your personal play area (above your board).
*   **Cubes:** Take as many Animal cubes from the reserve as there are spaces on the drafted card, and place one cube on each of those spaces.
*   **Limit:** You cannot hold more than **4 cards** simultaneously in your active area (including uncompleted Animal cards and any uncompleted Nature's Spirit card).
*   **Refill:** The display is refilled to 5 cards at the end of the turn.

#### Option B: Settle Animal (Unlimited Per Turn)
*   If the token arrangement on your board matches the pattern on an Animal Card in your hand:
    *   **Rotation Allowed:** The pattern shape may be rotated in any of the 6 hexagonal directions (0, 60, 120, 180, 240, 300 degrees). Mirroring/flipping is NOT allowed.
    *   **Height Match:** The token stack heights must match the pattern exactly.
    *   **Placement:** Take the **bottom-most** available Animal Cube from the card and place it on the specific token space indicated by the pattern.
    *   **Occupied State:** That token now hosts a cube. It still counts for scoring, but no further tokens can be placed on it, and it cannot host another cube.
    *   **Finality:** Once placed, a cube is never moved or removed, even if subsequent actions break the habitat pattern (e.g., by adding tokens to other spaces in that habitat).
    *   **Reuse:** A single token space can be part of multiple different Habitat patterns over time, but only one cube can occupy a single token.
    *   **Completion:** If the final cube is placed on a card, move the card to the side of your board. It is completed, **not discarded** (kept for end-game scoring), and no longer counts towards your 4-card hand limit.

#### Option C: Settle Nature's Spirit (Once Per Game)
*   If playing with the Nature's Spirit variant, you may settle your chosen Nature's Spirit:
    *   **Placement:** If your board matches the habitat pattern on your face-up Nature's Spirit card, move the Nature's Spirit cube from the card to the corresponding token within the habitat on your board.
    *   **Rules:** Follows the same placement rules as settling an animal (orientation, height match, occupied state, and finality).
    *   **Completion:** Once the Nature's Spirit cube is placed on your board, the card is completed, **not discarded** (kept for end-game scoring), and no longer counts towards your 4-card hand limit.

### Turn End / Refill Phase
At the end of your turn, perform the following in order:
1.  **Refill Tokens:** Draw 3 tokens from the bag to refill the empty space on the Central Board.
2.  **Refill Cards:** Refill the Animal Card display from the draw pile so there are exactly 5 face-up cards.

## 4. Scoring Rules

### A. Landscape Scoring
Landscape scoring occurs at the end of the game.

| Landscape          | Color       | Structure / Constraints                                                                 | Scoring Formula                                                                                                                               |
| :----------------- | :---------- | :-------------------------------------------------------------------------------------- | :-------------------------------------------------------------------------------------------------------------------------------------------- |
| **Tree**           | Green/Brown | Green on top. Stack: 1G (Bush), 1B+1G, or 2B+1G.                                        | **Bush (1G): 1 pt**<br>**Tree (1B+1G): 3 pts**<br>**Tall Tree (2B+1G): 10 pts**                                                                |
| **Mountain**       | Grey        | Stack of 1, 2, or 3 Grey. Must be adjacent to ≥1 other Mountain stack or score 0.       | **1 High: 1 pt**<br>**2 High: 3 pts**<br>**3 High: 7 pts**                                                                                    |
| **Field**          | Yellow      | No stacking. Groups of contiguous Yellow tokens.                                        | **5 pts per group** (Group must be ≥2 tokens).<br>*(Note: Larger groups do NOT score more points. Multiple small groups are optimal.)*        |
| **Building**       | Red         | Red on Red/Brown/Grey. Must touch ≥3 distinct colors (top surfaces of adjacent spaces). | **5 pts per valid Building** (adjacent spaces must have top tokens of ≥3 different colors out of the 6 total colors in the game, including red) |
| **Water (Side A)** | Blue        | No stacking. Longest path of consecutive blue tokens (River).                           | **2 tokens: 2 pts**<br>**3 tokens: 5 pts**<br>**4 tokens: 8 pts**<br>**5 tokens: 11 pts**<br>**6 tokens: 15 pts**<br>**+4 pts per token > 6** (River length is the number of tokens in the shortest path between its two ends; branching tokens not on this path do not count. Only your single best River scores.) |
| **Water (Side B)** | Blue        | No stacking. Create Islands.                                                            | **5 pts per Island** (An Island is a contiguous group of non-water spaces separated from other groups by blue tokens. You always have at least 1 island.) |

*Note on Trees: Brown tokens alone score 0 points. A single Green token (Bush) scores 1 point.*

### B. Animal Scoring
*   Animal Cards have a vertical bar of numbers (e.g., 2, 4, 7).
*   **End Game:** You score the **value of the topmost space without an Animal cube** on each card.
    *   *Example:* A card has 3 spaces with values 2 (bottom), 5 (middle), and 9 (top). You place 3 cubes on these spaces when drafting.
    *   If you settle 1 animal, you take the bottom cube (revealing the '2' space). The topmost space without a cube is '2', so you score 2 points.
    *   If you settle a 2nd animal, you take the middle cube (revealing '5'). The topmost space without a cube is '5', so you score 5 points.
    *   If you settle all 3 animals (completing the card), all spaces are empty. The topmost space without a cube is '9', so you score 9 points.
    *   If you settle 0 animals, all cubes remain on the card, and it scores 0 points.

### C. Nature's Spirit Scoring (Advanced Variant)
*   **Condition:** You only score points for your Nature's Spirit card if you successfully placed its Nature's Spirit cube onto your Personal board. If the cube remains on the card, it scores 0 points.
*   **Calculation:** Unlike Animal cards which have fixed points, Nature's Spirit cards score points at the end of the game based on the final layout of your Personal board.
*   **Scoring Types:**
    *   **Based on Landscape Count:** Points for the number of specific landscapes matching criteria (e.g., +4 points for each Mountain of height 2 and +4 points for each Mountain of height 3, including isolated Mountains).
    *   **Based on Connected Groups:** Points for groups of connected landscape tokens (e.g., +2 points for each group of 1 or 2 yellow tokens and +10 points for each group of 3 or more yellow tokens). An isolated landscape (size 1) counts as a group.
*   **Addition:** These points are in addition to normal landscape and animal card scoring.

## 5. Game End & Tiebreakers

### End Triggers
The game ends immediately when **either** condition is met:
1.  The Landscape Bag is empty when you need to refill the Central Board.
2.  At the end of a player's turn, that player has **2 or fewer empty spaces** (spaces with no tokens) on their Personal board.

### Final Round
*   Finish the current round so all players have equal turns.

### Winning
Sum points from:
1.  Landscapes (Trees, Mountains, Fields, Buildings, Water).
2.  Settled Animals (Highest visible value on each card).
3.  Nature's Spirit (If playing the variant and the cube was placed).
*   **Tiebreaker:** Player with the most settled Animal Cubes wins (including the Nature's Spirit cube if placed).

## 6. Solo Mode (Optional)
A single-player mode to beat high scores, measured in **Suns**.

### Setup
*   **Central Board:** Use the Solo side faceup (only 3 spaces for tokens).
*   **Animal Display:** Place only 3 Animal cards faceup in the display.
*   **Nature's Spirit Selection:** If playing with the Nature's Spirit variant, shuffle, deal 2 facedown, keep 1, place its cube on it, and discard the other (as in multiplayer).

### Turn Gameplay
*   **Refill Phase:** At the end of each turn, discard the remaining 6 tokens on the Central board back to the box (do not return them to the bag). Draw 9 new tokens from the bag to refill the 3 spaces (3 tokens each).
*   **Display Rotation:** If you did not draft an Animal card on your turn, you may discard 1 Animal card from the display and replace it with the top card of the draw pile.

### Scoring Suns
At the end of the game, calculate your final score in points, then determine your Sun rank:
*   **Base Suns (from Score):**
    *   **40+ pts:** 1 Sun
    *   **70+ pts:** 2 Suns
    *   **90+ pts:** 3 Suns
    *   **110+ pts:** 4 Suns
    *   **130+ pts:** 5 Suns
    *   **140+ pts:** 6 Suns
    *   **150+ pts:** 7 Suns
    *   **160+ pts:** 8 Suns
*   **Board Side Modifier:**
    *   **Side A:** +1 Sun
    *   **Side B:** +0 Suns
*   **Nature's Spirit Modifier:**
    *   **No Nature's Spirit (X):** +2 Suns
    *   **Connected Groups Spirit (Yellow Flower/Sprout shield icon):** +1 Sun
    *   **Number of Landscapes Spirit (Green Leaf/Sprout shield icon):** +0 Suns
