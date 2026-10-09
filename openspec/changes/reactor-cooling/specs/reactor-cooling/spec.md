# reactor-cooling

## ADDED Requirements

### Requirement: The reactor has a magnetic core and a coolant system

The reactor SHALL be two components: the magnetic core, whose integrity scales the thermal output, and the coolant
system, whose parts (two tanks, eight pipe segments, two core pumps, the heat exchanger and two radiator pumps) SHALL
each be a system with its own integrity and repair job.

#### Scenario: A hit on the hot leg
- **WHEN** a hit damages a hot leg pipe segment to 40%
- **THEN** the magnetic core's integrity is unchanged and the segment shows on the damage board as its own job

### Requirement: Damaged pipes and tanks leak coolant

A pipe segment or tank below 75% integrity SHALL leak coolant at 20 kg/s times its integrity's shortfall below 75%
over 75, the loop's flow SHALL fall by cavitation below 80% inventory to none at 60%, and an isolated segment SHALL
stop its leak.

#### Scenario: A segment at 0%
- **WHEN** a cold leg segment is at 0% and not isolated, with the makeup closed
- **THEN** the loop loses 20 kg/s and its flow starts to fall once the inventory is under 80%

### Requirement: Cooling is a load group on the engineering bench

The core pumps, radiator pumps and makeup pump SHALL be one load group, Cooling, on the engineering console's power
allocation, with a setpoint for their speed, priority 0 by default, and a breaker that stops them when opened.

#### Scenario: The Cooling breaker opened
- **WHEN** the engineer opens the Cooling feed's breaker at cruise
- **THEN** the loop's flow falls to 5% and the hot leg warms toward its scram limit

### Requirement: Losing the cooling heats the core

Coolant flow under 30% SHALL raise an alarm without scramming the core, so the core's blanket heats by its waste heat
until the blanket trip scrams it; and a scrammed core SHALL keep putting out afterheat, 6% of its thermal power at
the scram, falling with a 180 s time constant.

#### Scenario: The Cooling breaker opened at full power
- **WHEN** the engineer opens the Cooling feed's breaker with the core running
- **THEN** the core keeps running and its blanket climbs, until the hot leg over 380 K or the blanket over 820 K
  scrams it (at cruise, after about 3 minutes)

#### Scenario: Afterheat without cooling
- **WHEN** the core has scrammed and the loop's flow is still at natural circulation
- **THEN** the blanket keeps climbing for a while after the scram, and the reset is refused while the loop
  is at 350 K or over

### Requirement: Manual coolant control beats damaged automation

The engineering console SHALL offer manual control of pump speed, the chiller's bypass, the radiator pumps and the
makeup valve, and automation SHALL hold the cold leg at 327 K only by pump flow and radiator pumps, without
rebalancing the bypass or opening the makeup before 95%.

#### Scenario: One pump lost at full throttle
- **WHEN** core pump A is disabled with the reactor at 100% under automation
- **THEN** the hot leg drifts above 350 K, and a player in manual can bring it back into band
