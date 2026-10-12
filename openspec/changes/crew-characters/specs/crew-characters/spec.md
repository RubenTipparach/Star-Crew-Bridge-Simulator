## ADDED Requirements

### Requirement: Paired crew bodies
Every department SHALL have male and female adult cartoon variants, built from
shared topology with their dimensions held in data. Both variants SHALL retain
the same rig, garment, eye and avatar budget requirements.

#### Scenario: Selecting a body variant
- **WHEN** the owner changes Male or Female in the HTML viewer
- **THEN** the corresponding variant of the selected department is shown, or all six departments in the selected variant are shown in the lineup

### Requirement: Connected garments and fitted hair
The tunic SHALL have connected shoulders, sleeves and underarms. Trousers SHALL
join through a connected pelvis and crotch. Boots SHALL have distinct soles,
heels, toe boxes and ankles. The hair SHALL remain outside the scalp, and the
trousers SHALL stay clear of the visible tunic wall in rest and review poses.

#### Scenario: Checking repaired geometry
- **WHEN** the build reads the exported file and samples its five review clips
- **THEN** it rejects disconnected garments, garment self intersections, hair/scalp intersections and visible tunic/trouser intersections

### Requirement: Poseable Blender crew rig
Each character SHALL have an editable Blender body rig with hand and foot IK
targets, elbow and knee poles, IK/FK selection and separate finger curls. Clothes
SHALL follow the body through the supplied review poses. Exported GLBs SHALL
retain the 30-joint and 3,000-triangle limits and carry playable review clips.

#### Scenario: Posing an IK limb
- **WHEN** the animator enables IK and moves a hand or foot control in the saved Blender file
- **THEN** the corresponding two-bone limb follows the target with its elbow or knee directed by its pole

#### Scenario: Exporting the animation rig
- **WHEN** the build exports the character
- **THEN** Blender control bones are absent from the game skin and the actual file passes hierarchy, weight, clip and budget checks

#### Scenario: Inspecting animation in HTML
- **WHEN** the owner selects and scrubs a review clip
- **THEN** the actual exported skin deforms at that clip time and the optional skeleton follows it

### Requirement: Blocky figures remain until character approval
The engine and ship mockups SHALL use the blocky figures of `crew-npcs` section 7
until the owner explicitly approves a character design from named screenshots.

#### Scenario: Built but unapproved character
- **WHEN** a character has been built but its screenshots have not been approved
- **THEN** the bots aboard are still drawn as the existing blocky figures

### Requirement: Data-driven cartoon proportions and geometric eyes
A crew character SHALL use data-held exaggerated head, hand and foot sizes and
short neck proportions. Each eye SHALL have an iris width of 0.45-0.58 of the eye
width and a pupil width of 0.40 of the iris width, with both boundaries as geometry.

#### Scenario: Eye ratios read from a built character
- **WHEN** the build writes a character mesh
- **THEN** it prints iris and pupil ratios measured on the exported mesh and refuses values outside the required ratios

### Requirement: Uniform garments cover the body
The tunic, trousers and boots SHALL be geometry over the body, with their own hem,
cuff, collar and trouser-leg edges, and no covered skin showing through in rest pose.

#### Scenario: Rest-pose review
- **WHEN** the character is rendered from front, back and both sides
- **THEN** no skin is visible inside the tunic or trouser outlines

### Requirement: Avatar budget and probe lighting
A crew character SHALL have at most 3,000 triangles, 30 bones, one material and one
avatar draw, read from its built file. Adopted characters SHALL take nearby deck
light probes in normal, red-alert and emergency lighting states.

#### Scenario: Over-budget built file
- **WHEN** a written character file holds more than 3,000 triangles
- **THEN** the build stops and names the character and its measured triangle count

#### Scenario: A moving approved character
- **WHEN** an approved character moves between deck light probes
- **THEN** its albedo is multiplied by the interpolated probe lighting around it

### Requirement: Interactive character review
The local HTML review page SHALL load the Blender-built GLBs, preserve pending
approval, support character selection and orbit/pan/zoom, and expose front, side,
back, eye, wireframe and illustrative lighting views. It SHALL measure visible
frame triangles and draws from the renderer separately from built-file counts.

#### Scenario: Reviewing from disk
- **WHEN** the owner opens the inlined HTML file with the pinned three.js CDN available
- **THEN** the models and review controls work without a local asset server

#### Scenario: Reviewing an unapproved model
- **WHEN** the owner selects a character or changes its review view
- **THEN** its status remains pending and the engine and ship occupants stay blocky

#### Scenario: Reviewing lighting
- **WHEN** the owner selects normal, red-alert or emergency review light and orbits the camera
- **THEN** the world-axis illustrative probe light multiplies exported albedo without changing the built GLB
