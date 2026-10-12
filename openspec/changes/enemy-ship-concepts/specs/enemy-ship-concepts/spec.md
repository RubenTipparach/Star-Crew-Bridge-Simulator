## ADDED Requirements

### Requirement: Three sequential enemy concept studies
The design review SHALL present three distinct enemy ship concepts, each with a retained initial floor plan, a hull developed around that plan, and a fitted floor plan.

#### Scenario: Compare the process
- **WHEN** the owner selects a ship and changes design stages
- **THEN** the initial rooms, hull silhouette and fitted rooms are inspectable in the same metre coordinate system

### Requirement: Hull and room agreement
The fitted study layouts SHALL keep every room inside its occupied hull envelope, with at least 0.45 m clearance, 2.8 m headroom, aligned doors and a connected crew route.

#### Scenario: Inspect a shaped compartment
- **WHEN** the owner selects a room on a fitted deck
- **THEN** the plan and cutaway show the same polygon, floor height, area and purpose

### Requirement: Reviewable rough artwork
The HTML review SHALL use simple existing material layers and flat-shaded geometry, provide free orbit and deck cutaways, and display its measured geometry and texture budget.

#### Scenario: Inspect all concepts
- **WHEN** the owner opens the inlined HTML file from disk
- **THEN** all three concepts and design stages work without a local server, with only the pinned Three.js CDN dependency

#### Scenario: Inspect interior scale
- **WHEN** the owner opens a fitted cutaway
- **THEN** the rooms, clear door openings and human scale references are visible, with normal and red-alert lighting available
