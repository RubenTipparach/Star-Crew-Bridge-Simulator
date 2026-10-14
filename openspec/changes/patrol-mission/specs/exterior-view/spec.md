# exterior-view

## ADDED Requirements

### Requirement: Every window shows the real exterior

Every window of every deck and every viewscreen SHALL show the exterior state of the frame: the sky from the system
the Tern is in, the stations, ships, craft, missiles, bolts and bursts the server sends, and the Tern's own hull, all
from one ship pose a frame, never a fixed backdrop.

#### Scenario: The fight from the medbay
- **WHEN** a crew member stands at the medbay's window during the encounter
- **THEN** the window shows the sky turned by the Tern's attitude and the Hound, its fighters and their bolts where the
  server has them

#### Scenario: The relay where the helm flies
- **WHEN** helm's COURSE puts the bow on the drill's waypoint
- **THEN** the bridge's windows and the viewscreen show the relay there with its beacon blinking

### Requirement: One star lights everything outside

The star's direction of the frame SHALL light the sky's disc and the planets' day sides, the Tern's hull (its baked
light divided out and the frame's multiplied in) and every ship, craft and station outside, through one lighting rule.

#### Scenario: The Tern rolls in the fight
- **WHEN** the Tern rolls 90 degrees
- **THEN** the lit side of its nacelles seen from the side windows turns with the sky's sun

#### Scenario: The bake's own sun leaves the hull as baked
- **WHEN** the frame's sun in the ship's frame is the direction the hull was baked with
- **THEN** the hull is drawn with its baked colours unchanged

### Requirement: The windows follow the patrol

Through the patrol the windows SHALL show the berth's cradle and the starbase while docked, the cradle sliding past as
the Tern backs out, the station receding on departure, streaks through every window at a jump, and the bodies of the
waypoint after it, sized and placed from the system file.

#### Scenario: Arriving at the Pell beacon
- **WHEN** the Tern arrives at the Pell beacon
- **THEN** Orrin fills a disc about 17 degrees in radius in the windows facing it and the star's disc is smaller than
  at Tamsin
