# star-systems

## ADDED Requirements

### Requirement: Every body in a system file has a kind

Every body in a system file SHALL have a `kind` of `rocky`, `gas_giant` or `asteroid`, consistent with its radius, and a
moon SHALL name its `parent`. Belts SHALL be listed apart from bodies in `belts`.

#### Scenario: A body without a kind
- **WHEN** a system file lists a body with no `kind`
- **THEN** the loader refuses the file, naming the body's path and the field

### Requirement: Each star system has a unique background

No two system files SHALL name the same `sky`, and the sky each names SHALL exist.

#### Scenario: Two systems with one sky
- **WHEN** two files in `data/space/systems/` both name the sky `bloom`
- **THEN** the data check fails naming both files

### Requirement: Each star system has at least one station

A system file SHALL list at least one station.

#### Scenario: No station
- **WHEN** a system file's `stations` is empty
- **THEN** the loader refuses the file

### Requirement: Each star system has several rocky worlds, a gas giant and multiple asteroids

A system file SHALL have at least 3 bodies of kind `rocky` (rocky moons counted), at least 1 of kind `gas_giant` and at
least 2 of kind `asteroid`.

#### Scenario: Not enough asteroids
- **WHEN** a system file has one asteroid
- **THEN** the loader refuses the file, naming the rule

#### Scenario: The Shoals after its delta
- **WHEN** the Shoals' file has Cinder, Tamsin and Pell rocky, Orrin a gas giant, and Skerry, Holm and Wick asteroids
- **THEN** it loads
