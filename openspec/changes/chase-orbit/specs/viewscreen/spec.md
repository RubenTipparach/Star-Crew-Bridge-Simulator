# viewscreen

## ADDED Requirements

### Requirement: Ships are drawn on the server's clock

The client SHALL draw the ships between the two snapshots whose server ticks bracket a playback clock that advances by
the client's frame step, so the rendered turn of a ship turning at a steady rate advances evenly from frame to frame.

#### Scenario: Two snapshots in one frame
- **WHEN** one frame of a single-player game runs two server ticks and receives two snapshots
- **THEN** the ship drawn on the next frames moves by even steps, not a jump and a pause

### Requirement: The chase camera follows the ship's turn smoothly

With CHASE on, the viewscreen's camera SHALL follow the ship's rotation with a critically damped lag in the system
frame, its time constant from data, giving the same pose for the same elapsed time at any frame rate and never turning
past the ship. A change of camera, CHASE or ORBIT SHALL ease from the old picture to the new over a time from data.

#### Scenario: The ship turns
- **WHEN** the Tern turns at a steady rate with CHASE on
- **THEN** the camera settles behind the turn by the time constant times the rate, and the ship is seen to swing in the
  picture

#### Scenario: CHASE pressed
- **WHEN** CHASE is pressed
- **THEN** the picture moves from the bow camera to the chase camera over the blend time instead of cutting

### Requirement: ORBIT turns the chase camera round the ship

The camera widget SHALL have an ORBIT button, available while CHASE is on. With ORBIT on, a drag on the viewscreen
SHALL turn the camera round the ship, all the way round and up and down short of the poles, the wheel or a pinch SHALL
move it in or out within limits from data, and the orbit SHALL keep its place relative to the ship as it flies.
Turning ORBIT off SHALL ease back to the chase camera.

#### Scenario: Drag across the viewscreen
- **WHEN** ORBIT is on and the player drags across the viewscreen
- **THEN** the camera circles the ship and the bracket on the target follows the picture

#### Scenario: The captain orbits
- **WHEN** the captain orbits his viewscreen
- **THEN** the bridge's main viewscreen shows his orbit
