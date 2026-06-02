# Harmonies BGA Bot Research Notes

## BGA Game State Structure

BGA stores the entire game state in `window.gameui.gamedatas`.

### Key Elements of `gamedatas`:
* **`players`**: Object containing players keyed by player ID.
  * `tokensOnBoard`: Object mapping cell keys (e.g. `cell_[playerId]_[col]_[row]`) to arrays of token objects (allowing stacking up to 3 height).
    * Token structure: `{ id, location, location_arg, type, type_arg, done }` where `type_arg` represents the color index.
  * `boardAnimalCards`: Drafted animal cards currently on the player's board.
  * `animalCubesOnBoard`: Cubes placed on the board.
  * `emptyHexes`: Counter of remaining empty spaces.
* **`river`**: Array representing the face-up animal cards display.
  * Card structure: `{ id, pointLocations, pattern, isSpirit, type_arg }` where `type_arg` maps to the card index in the sprite sheet.
  * `pattern`: Array of hex offsets defining the shape: `{ colors: [type_arg_list], position: hex_offset, allowCube: boolean }`.
* **`tokensOnCentralBoard`**: Object keyed by space ID (`1` to `5`) containing arrays of 3 tokens available for drafting.
* **`gamestate`**: The active BGA state machine state.
  * `active_player`: Current active player ID.
  * `args`: Active state arguments.
    * `placeAnimalCubeArgs`: List of valid cell placements for animal cubes.
    * `possibleCards`: List of cards valid for settling.

---

## Constants Mapping

### Landscape Token Colors (`type_arg` mapping)
* **`1`**: Blue (Water) - *Offset 0% (top of `coloredTokens.png`)*
* **`2`**: Grey (Mountain) - *Offset 60%*
* **`3`**: Brown (Wood/Trunk) - *Offset 20%*
* **`4`**: Green (Leaf/Foliage) - *Offset 40%*
* **`5`**: Yellow (Wheat/Field) - *Offset 100% (bottom of `coloredTokens.png`)*
* **`6`**: Red (Brick/Building) - *Offset 80%*
* **`7`**: Virtual code representing a completed Red Building stack (functionally treated as matching Red).

### Animal Cards (`type_arg` mapping)
All 42 card patterns (32 standard animals + 10 spirit cards) are fully documented in the static JSON database:
* [cards_database.json](file:///X:/Programming/Python/Projects/Web%20&%20SPAs/Harmonies%20-%20BGA%20Bot/docs/cards_database.json)

Card `type_arg` corresponds to the design ID in the catalog, matching layout positions in [animalCards.jpg]("x:\Programming\Python\Projects\Web & SPAs\Harmonies - BGA Bot\docs\animalCards.webp") (10 columns by 5 rows).

---

## Grid System & Stacking Representation

### 1. Grid Coordinates
* Cell keys in JS model: `cell_[playerId]_[col]_[row]` (e.g. `cell_99795824_2_2`).
* Board Side A vs B determined by `gamedatas.boardSide` (`"sideA"` or `"sideB"`).
* Board coordinate schema defined in `gamedatas.hexes` as a list of `{ col, row }` coordinates.
* **Hex adjacency rule (odd-r layout):**
  * Neighbors of hex `(col, row)` depend on whether `row` is even or odd.
  * Even row neighbors: `(c-1, r-1)`, `(c, r-1)`, `(c-1, r)`, `(c+1, r)`, `(c-1, r+1)`, `(c, r+1)`
  * Odd row neighbors: `(c, r-1)`, `(c+1, r-1)`, `(c-1, r)`, `(c+1, r)`, `(c, r+1)`, `(c+1, r+1)`

### 2. Token Stacking representation
Inside `tokensOnBoard[cell_key]`, tokens are ordered in a list representing the stack bottom-to-top.
* `location_arg` of each token matches its level:
  * `1` = Level 1 (bottom)
  * `2` = Level 2 (middle)
  * `3` = Level 3 (top)
* Color of token at each level is determined by its `type_arg` color mapping.

### 3. Animal Cube tracking
* Cubes are registered globally in `cubesOnAnimalCards`.
* **State 1: On Card**
  * `location` = `card_[cardId]` (e.g. `card_14`).
  * `location_arg` = slot index on the card's progress track (lower values are bottom-most, matching lower index).
* **State 2: On Board**
  * `location` = `cell_[playerId]_[col]_[row]` (e.g. `cell_99795824_2_2`).
  * `location_arg` = stack level (e.g. `3` means it sits on top of a 3-high token stack).
  * Presence of cube in `cubesOnAnimalCards` with a `cell_` location means that hex is locked/occupied. No more tokens can be placed or stacked there.

### 4. Animal Card Pattern Chains
* Pattern shape validation is stored as an array of step objects in `card.pattern`.
* The shape is built as a **relative path chain**:
  * **Step 0 (index 0):** The start coordinate of the pattern. `position` is always `0` (represents base offset `(0,0)`).
  * **Step N (index > 0):** `position` values (`0` to `5`) represent the direction offset relative to the **previous step's** hex in the list.
* Direction indices `0` to `5` map to neighbors depending on whether the current step's row is even or odd (standard odd-r grid offsets).
  * Example pattern containing duplicate `position: 3` values (e.g. `[Step 0 (pos 0), Step 1 (pos 3), Step 2 (pos 3)]`) forms a straight-line sequence of 3 hexes (Start -> Neighbor 3 -> Neighbor 3 of Step 1).


