## ADDED Requirements

### Requirement: Model-specific UV artwork
The player exterior SHALL use original hull artwork with seams and livery projected on the actual 3D hull, inspected there, and baked into its UV layout before texture enhancement. The editable projection, unenhanced bake, region masks, UV guide and image sources SHALL be retained.

#### Scenario: Inspect the painted hull
- **WHEN** the player hull is shown with its UV guide
- **THEN** armour features follow named UV islands and the approved warp attachment dimensions remain unchanged

### Requirement: Connected interior windows
The full-detail player exterior SHALL contain genuine window openings aligned to the interior layout used in the preview, with rooms, furniture and crew at their existing metre scale.

#### Scenario: Look through a window
- **WHEN** the preview camera is outside a command-suite window
- **THEN** the room and its furnishings are visible through the hull opening and the interior portal without an opaque picture covering the aperture

#### Scenario: Inspect the window from an angle
- **WHEN** the viewer orbits either side of the command hull
- **THEN** physical pale-silver rims surround the actual apertures, the side paint contains no false windows, and the command hull has no projecting shoulder blades or crown

#### Scenario: Inspect deck scale
- **WHEN** the viewer selects the cutaway or interior view
- **THEN** the preview shows the actual deck heights and compartment outlines with normal and red-alert lighting options

#### Scenario: Inspect window thickness and seams
- **WHEN** the viewer looks through any command-suite window from outside
- **THEN** its room wall is at most 0.40 m behind the outer hull, the lining meets the hull without gaps, and no unrelated mechanical texture strip appears on the cut edges

#### Scenario: Inspect windows on the other decks
- **WHEN** the viewer selects deck B or C in the exterior preview
- **THEN** ten deck B and six deck C windows open into the corresponding furnished compartments, with at most 0.65 m returns, and their frames and dark glazing remain visible in both lower-detail textures

#### Scenario: Look out from the mess hall
- **WHEN** the mess hall is viewed from inside or outside
- **THEN** its three window centers sit 1.50 m above the floor, with 1.70 x 0.80 m clear openings and frames fitted to the actual exterior surface

### Requirement: Reusable player exterior
The asset pipeline SHALL produce a textured exterior GLB for the Tern from its layout hull, with two named warp pylons and two separated warp nacelles in ship-local metres.

#### Scenario: Inspect the player asset
- **WHEN** the exterior asset is built and loaded in the inspection mockup
- **THEN** its hull follows the approved profile, both pylons and nacelles are visible, and deck heights and existing doors retain their source positions

#### Scenario: Inspect the revised warp silhouette
- **WHEN** the player ship is viewed from the bow or side at any LOD
- **THEN** both pylon roots and tips share the hull's middle height, and the 56 m nacelles remain below the main dorsal hull height

#### Scenario: Inspect the rounded bow
- **WHEN** the Tern is viewed from the side, top or bow at any detail level
- **THEN** the lower bow follows the bridge's enlarged curved outline with a softened front rim, its sensor and missile mouths meet that face, and the torpedo room, magazine and command rooms remain inside the hull

#### Scenario: Inspect the quieter livery
- **WHEN** the player ship is shown in full detail or either LOD
- **THEN** broad warm-silver panels, subtle variation and restrained slate and livery-colored accents define the hull and fittings, with a narrow grey equatorial service band and no dense generic mechanical pattern

### Requirement: Custom exterior and LOD textures
The exterior assets SHALL use custom Material Maker paint textures without a repeating square panel grid, and medium and distant player models SHALL include distinct baked texture atlases from the detailed exterior.

#### Scenario: Inspect detailed hull artwork
- **WHEN** the revised exterior is inspected from above, below or alongside
- **THEN** fitted original artwork shows varied armour plating, service recesses, vents and framed window openings, and both LOD atlases retain the same surface design

#### Scenario: Inspect lower detail textures
- **WHEN** the medium or distant player GLB is loaded independently
- **THEN** its embedded atlas preserves hull livery, radiator detail and registration without requiring the full-detail mesh or textures

### Requirement: Budgeted exterior models
The asset pipeline SHALL refuse player exteriors above 12,000 triangles at full detail, 3,000 triangles at medium detail or 600 triangles at distant detail.

#### Scenario: Build all player detail levels
- **WHEN** the generator exports the player's three detail levels
- **THEN** the manifest records actual triangle counts below each ceiling and the model read-back agrees

### Requirement: Shared textured exterior concepts
The exterior design SHALL include two companion silhouettes using the same material sources and generator, presented as exterior concepts without implied interiors or gameplay.

#### Scenario: Review the fleet
- **WHEN** the inspection page displays the fleet
- **THEN** the Tern, Osprey courier and Shrike raider have distinct silhouettes and inspectable textured models

### Requirement: Reviewable reproducible art
The asset pipeline SHALL retain a committed generator, material provenance, an editable Blender scene, reviewed captures and a check mode that compares the actual generated GLBs with the delivered files.

#### Scenario: Rebuild delivered models
- **WHEN** check mode runs with the recorded Blender version
- **THEN** the generated GLBs match the delivered bytes and all geometry and texture checks pass

### Requirement: Angular armor and selectable liveries
The player exterior SHALL retain its accepted silhouette while displaying angular white armor with narrow side seams, retained dorsal mechanical breaks, and registration painted directly onto the hull. It SHALL provide Copper, Cyan and Rescue liveries, each with matching full, medium and distant textures within the existing per-model budgets.

#### Scenario: Open the preferred livery
- **WHEN** the exterior inspection page first opens
- **THEN** the Tern wears cyan blue painted markings and matching fittings, with the Cyan livery selected

#### Scenario: Inspect armor and registration
- **WHEN** the player exterior is inspected close to the bow and dorsal registration
- **THEN** panel boundaries are straight and chamfered, the grey subhull is visible between armor sections, and the registration has no raised plate or rectangular backing

#### Scenario: Compare liveries at each distance
- **WHEN** a livery is selected and the inspection page switches between all three detail levels
- **THEN** its markings and armor breaks remain consistent, and the full-detail windows remain aligned with the same furnished interiors

#### Scenario: Inspect the retained mechanical detail
- **WHEN** the Tern is inspected from above or below
- **THEN** the retained grey channels contain pipes, couplings and mechanical detail, with calm white armor around them
- **AND** both lower-detail textures preserve those details and the narrow side-panel seams

#### Scenario: Armor remains the dominant forward surface
- **WHEN** the Tern is viewed from either forward quarter in any livery
- **THEN** broad ivory armor panels SHALL cover the command walls and bow, separated by fine seams with only a narrow equatorial grey band and small recessed pipe detail.

#### Scenario: Plate the sides and wrap the livery
- **WHEN** the forward hull is viewed across roof and wall transitions
- **THEN** normal ivory hull panels with narrow seams SHALL cover the side walls, and the livery stripe SHALL continue onto the vertical surfaces.
- **AND** its layout SHALL originate from inspected 3D hull projections baked into UV space before texture enhancement and LOD baking.

#### Scenario: Follow a panel seam across a roof edge
- **WHEN** the roof, shoulder bevel and adjacent side wall are inspected together
- **THEN** each transverse panel seam SHALL cross their shared edges at the same ship-local station without a jump or a second offset seam.
- **AND** the actual baked seam crossings SHALL be checked through both receiving UV faces after texture enhancement.

#### Scenario: Follow the complete livery segment
- **WHEN** the Tern roof and side paint are inspected together
- **THEN** the livery bands SHALL meet in a flat 9 m central segment with parallel edges, then wrap onto both walls with aligned boundaries and subtle paint variation.

#### Scenario: Inspect the equatorial split
- **WHEN** the hull is viewed from either side or the bow
- **THEN** a narrow grey service band SHALL divide the upper and lower armor without blocking windows or dominating the white plating.

#### Scenario: Paint follows the armor material
- **WHEN** the livery is inspected close to the roof and wall joins
- **THEN** panel seams and subtle surface finish SHALL remain visible through the painted area, with grey mechanical recesses retained.
