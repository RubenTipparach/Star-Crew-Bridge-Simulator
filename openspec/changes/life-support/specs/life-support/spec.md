# Life Support

## Purpose

The ship's air: gas in every compartment as moles, pressure and temperature, moved by pressure
through the openings of the one compartment graph, mixed by doors and ventilation, made and cleaned
by a powered plant, breathed by the crew, lost through breaches, pumped out of the bays and the
airlock, and shown on consoles by the model that computes it.

## ADDED Requirements

### Requirement: Each compartment holds gas as moles and energy
Each compartment and the air duct SHALL hold moles of oxygen, nitrogen, carbon dioxide and smoke and
an internal energy, with temperature from the energy over the gas's and the fittings' heat capacity
(4,000 J/K per m^3) and pressure by the ideal gas law. Space SHALL be a node at 0 Pa. Standard air
SHALL be 101.3 kPa at 294.15 K with 20.9% oxygen.

#### Scenario: The ship at rest
- **WHEN** the ship cruises for 30 minutes with eight crew working
- **THEN** every compartment reads about 101.2 kPa with oxygen at about 21.2 kPa and CO2 under 0.05%

### Requirement: Gas moves through openings by pressure, implicitly and without negative amounts
Every open link SHALL pass gas by the compressible orifice equation, choked below a pressure ratio of
0.528 and linear below 20 Pa of difference. Each sub-step SHALL solve the end-of-step pressures
implicitly over all links at once and SHALL move gas upwind in order of falling pressure, so that no
node holds negative gas and a draining room's pressure never rises above 1 Pa.

#### Scenario: A pod blown open
- **WHEN** a 2.2 m^2 breach opens in a 15.6 m^3 turret pod
- **THEN** the pod passes 6.3 kPa within 0.4 s and no species in any node goes below zero

#### Scenario: Accuracy at 10 Hz
- **WHEN** a 1 m^2 breach empties the hangar at 10 Hz and at 1 kHz
- **THEN** the times to 50 kPa and to 6.3 kPa agree within 0.3 s

### Requirement: Decompression times follow volume and area
With its doors shut, a compartment breached to space SHALL lose pressure by the flow equation alone,
giving the times of the design's decompression table within 5%.

#### Scenario: The bridge holed
- **WHEN** a 1 m^2 breach opens in the bridge (462 m^3)
- **THEN** it falls below 50 kPa in about 3 s and below 6.3 kPa in about 11 s

#### Scenario: A small leak in the hangar
- **WHEN** a 0.1 m^2 breach opens in the hangar (1,698 m^3)
- **THEN** it takes about 97 s to fall below 50 kPa and about 395 s to fall below 6.3 kPa

### Requirement: Doors keep each compartment its own pressure boundary
Doors, hatches and ladder hatches SHALL be shut unless a crew member is passing or the damage
control board holds them. A door SHALL refuse to open across more than 20 kPa of difference unless
overridden. A door between two compartments SHALL close itself when either side is below 85 kPa and
falling faster than 1 kPa/s, unless held. Doors to space SHALL open and close only on command.

#### Scenario: A breach while someone is in the doorway
- **WHEN** a 1 m^2 breach opens in the quarters while their door to the main corridor is open
- **THEN** the door closes itself within 3 s and the corridor keeps most of its air

#### Scenario: Pulling someone out
- **WHEN** a crew member tries to open the medbay's door with the medbay at vacuum
- **THEN** the door refuses ("interlock: 101 kPa across"), and opens only when the override is held

### Requirement: Ventilation trips on a leak and refills afterwards
Every room SHALL exchange air with the duct at its design air changes times the fans' supply ratio.
A room's damper SHALL shut and latch when its net flow exceeds 0.02% of the room's gas a second
(unless it is refilling a rising room), when the room is below 85 kPa, its smoke above 2,000 ppm,
the duct below 85 kPa, or the fans stopped. A latched damper SHALL reopen after its room has stopped
falling for 30 s if the room is within 3 kPa of the duct, refill the room if it is above 10 kPa, and
otherwise stay shut until the board acts. A refill SHALL stop if the room falls while it runs.

#### Scenario: The duct does not feed a leak
- **WHEN** a 0.1 m^2 breach opens in the hangar
- **THEN** the hangar's damper shuts within 2 s and the duct stays above 100 kPa

#### Scenario: Refilling a patched room
- **WHEN** a breach in the quarters is patched at 50 kPa
- **THEN** the quarters are refilled from the duct to 100.3 kPa automatically, using reserve gas

### Requirement: The plant makes and cleans the air with power
The oxygen generator SHALL hold the duct's oxygen at 21.2 kPa, drawing 0.8 MJ per mol of oxygen up
to 1.25 mol/s; the scrubbers SHALL remove 80% of the CO2 and 90% of the smoke in 4 m^3/s of duct
air; thermal control SHALL condition the supply air to 20 C within 1.2 MW of cooling and 0.6 MW of
heating; make-up SHALL hold the duct at 101.3 kPa from the reserves at up to 40 mol/s. Each SHALL
deliver in proportion to the supply `power-grid` gives it.

#### Scenario: Thermal control lost in combat
- **WHEN** thermal control loses its power during combat
- **THEN** the turret pods pass 30 C within about 8 minutes

### Requirement: Crew are affected by the air they breathe
Each unsuited crew member SHALL consume oxygen and produce CO2 and heat (0.304 and 0.264 mmol/s and
100 W at rest, 2.5 times the gases and 300 W working) and SHALL become impaired, unconscious and dead
by oxygen partial pressure, CO2 partial pressure, smoke dose, heat, cold and pressure as the data's
tables give, including death after 90 s below 6.3 kPa.

#### Scenario: Caught in a breached bay
- **WHEN** an unsuited crew member is in the port launch bay when a 1 m^2 breach opens
- **THEN** they are unconscious within about 12 s and dead within about 96 s unless pulled out

### Requirement: Bays pump down into a receiver and launch at the pumps' stop
The bay pumps SHALL move a bay's air into the receiver at 24 m^3/s of displacement with power by
isothermal compression, refuse to start with an unsuited crew member in the bay, stop at 5 kPa or a
full receiver, and then open the bay's vent valve and permit launch. Repressurization SHALL return the
receiver's air and finish from the duct.

#### Scenario: Pumping down a launch bay
- **WHEN** flight ops pumps down the port launch bay
- **THEN** launch is permitted after about 29 s, about 97% of the bay's air is in the receiver, and
  the pumps drew at most about 3.7 MW

#### Scenario: Pumping down the hangar
- **WHEN** the hangar is pumped down for the shuttle
- **THEN** the pumps stop at 5 kPa after about 208 s, having used about 835 MJ

#### Scenario: Emergency vent
- **WHEN** a launch bay's drop door is opened with the bay full
- **THEN** the bay passes 6.3 kPa about 1 s later and loses about 277 kg of air

### Requirement: The airlock cycles in under a minute
The airlock SHALL cycle out by pumping into cargo to 5 kPa and opening its outer door, and cycle in
by shutting its outer door and equalizing through its valve before opening the inner door.

#### Scenario: Going outside
- **WHEN** a suited crew member cycles out
- **THEN** the airlock reaches 5 kPa in about 36 s and the outer door opens 3 s later

### Requirement: Readouts and previews come from the model
Every life support readout SHALL be the state of the last sub-step, and every preview (time to a
pressure, a refill's time and gas cost, time to Armstrong's limit) SHALL step the same functions on
a copy of the compartment, its pump and its store.

#### Scenario: A pump-down preview
- **WHEN** flight ops sees "launch in 29 s" for the port launch bay and starts the pump-down
- **THEN** launch is permitted within one sub-step of 29 s later

### Requirement: Atmosphere data is validated at startup
`data/ships/<ship>/atmosphere.json` SHALL be validated at startup with the rules of
`power.json`: an unknown key, a non-finite number or an id that does not exist SHALL stop startup with
the file and the field.

#### Scenario: A store in a missing room
- **WHEN** a reserve bottle names a compartment the layout does not have
- **THEN** the game refuses to start and names the file, the store and the compartment
