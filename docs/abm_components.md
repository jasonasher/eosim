# A Component Architecture for Complex Simulation

The purpose of this document is to describe some design considerations for
creating complex simulations out of interacting, loosely-coupled, and reusable
components.

The primary use case for this design is the construction of agent-based models for
disease transmission, but the approach is applicable in a wide array of
circumstances.

## Core Concepts

### Context and Components
The Components of the simulation interact with each other indirectly through
a Context that holds the current state of a given simulation. The Context
provides methods for accessing and mutating this state. Components generally
take turns having focal control of the simulation with a mutable reference to the
Context whereby they handle their respective concerns for the simulation.

Components should be designed to be minimally coupled - they do not directly
access other Components and only use the mechanisms afforded by the Context to
communicate with each other.

In a typical simulation Components represent independent areas of concern.
For example in a simple model of disease transmission, there may be separate
Components for population loading, disease progression, and disease transmission.
If Components are properly designed it is possible to modify the simulation behavior
dramatically by changing only a few Components and leaving the rest as-is.

### Context Plugins
The Context provides a mechanism to store and retrieve data via a pluggable
architecture. These plugins may also provide an interface to manipulate the data
and provide other useful capabilities that extend the base Context.

These plugins will be the primary way that simulations can be augmented to support
new concepts that are required to implement a given problem. For example, a plugin
may introduce the concept of a person - an entity with some unique id - and methods
for creating new people or removing existing people from the simulation. Other plugins
may provide the ability to associate typed key-value pairs for each person in a simulation
together with mechanisms for setting and retrieving these pairs.

### Callbacks
Multiple aspects of Context interaction will make use of the Callback pattern.
Components can contribute functions or closures to the Context that will be called
in certain circumstances - such as at future points in time or in response to specific
events.

### Planning
The Context provides a mechanism for Components to indicate that they would like
to have the opportunity to have focal control of the simulation at a future point
in time. These plans are commonly implemented via an time-ordered queue of Callbacks.

Components may need to retrieve plans that they have created and modify or cancel
them. Additionally, Components may schedule plans for the same point in time, and
the Context should provide a deterministic method for tie-breaking, and potentially
mechanisms to manipulate this tie-breaking process for certain behaviors (such as
requesting that some callback be exectuted first among all plans at the same
point in time).

### Callback Queue
The Context provides a mechanism to queue Callbacks to be executed at the current time,
prior to time moving forward to the next plan. This queueing process enables Components
to typically ignore the direct behavior of other Components as other Components will
generally wait in the queue for their turn to manipulate the Context instead of
being interleaved with the current focal Component.

### Events
The Context provides a mechanism for Components or Plugins to release transient
Events and to register to handle Events that occur. Events are typed and
registration is specific to the Event type. Registering to handle an event entails
contributing a Callback that has an appropriate signature to process the typed Event.
The Context may provide additional mechanisms to restrict the circumstances under
which an event handling registration may be invoked.

Most Components will handle these events by having a Callback added to the event queue,
in essence delaying their processing of a given event until it is their 'turn'.
However, some Components or Plugins may need to handle Events immediately in order
to maintain internal consistency.

For example, consider a Component that is trying to count
how many people have different values of some property. That Component would likely
register to handle all events where that property changes in order to keep the counts
up to date. Suppose another Component modifies the value of that property twice
while it has focal control - first from A to B and then from B to C. This would
cause two Callbacks to be queued for the tracking Component. When the first of these
Callbacks is processed the Component will see that the property changed, but also
that the current value is C. The designer of the Component will be uncertain how
to adjust its counts to be consistent with the true state of the simulation.
If it only adjusts the counts by decrementing the counts for A and incrementing
for B then it will not represent the current simulation. But, it also will not
know what other callback proccessing is waiting to be performed in the queue and
cannot assume that decrementing the counts for A and incrementing those for C is
correct either. Thus in general these kinds of tracking Components or Plugins will
need to handle Events immediately.

Note that while simulations can be constructed where Components always handle Events
immediately, this can lead to intertwined code that is difficult to understand and
debug.

### Time and Simulation Execution
The Context keeps track of the current time in the simulation which never decreases.
The main simulation loop progresses by first processing the callback queue until
it is empty, then advancing time to the time of the next plan in the planning queue,
and processing that plan. Both plans and queued callbacks may add additional callbacks
or plans to the respective queues. In this way the simulation alternates between
processing the callback queue and planning queues until both are empty, at which
point the simulation terminates.


## Design Choices

### Scale
The simulation will support moderately-large simulations involving millions to
tens of millions of agents - typically enough to cover detailed simulations of a
US state or metropolitan statistical area. Simulations with hundreds of millions
or billions of agents may require different techniques to be adequately performant.

### Parallelism
This design assumes that most users of a simulation will be interested in executing
multiple replicates of a given simulation to explore stochastic and structural
uncertainties of a problem of interest. This design further assumes that
within-simulation parallelism is less important than across-simulation
parallelism at the scale of problems being modeled. And, as most model builders
will be authoring and contributing Components to a Simulation this lowers the
barrier to entry as builders will not need to concern themselves with threading.

### Time
The design models time with floating-point values instead of coarse discrete
time steps. Infectious disease transmission and disease progression processes
can have events occuring at a variety of temporal scales and it is most convenient
to represent time generally without concern for the effects of discretization.

### Components as Actors
This design presumes that Components will represent larger scale volitional processes
in a simulations and not individual people or agents. As such the number of Components
in a simulation will generally be much smaller than the number of people. People
are typically modeled as data records with state that is modified by one or more
Components that each have a defined area of concern.


## Common Plugins
The following is a list of plugins that provide core capabilities that are commonly
used by many simulations build in this framework. A minimal framework implementation
may not implement all of these and instead rely on more basic Context features to
implement the relevant capabilities.

### Global Properties
This plugin defines typed key-value pairs that represent parameters in the simulation.
The plugin provides mechanisms for retrieving and setting the value of the
respective parameters in the simulation. It also provides a mechanism for Components
to provide Callbacks to handle whenever global properties change. This mechanism
can entail releasing appropriate Events that indicate a global property has changed
and capturing its previous value.

### People
This plugin defines the concept of a person as an entity with some unique identifier,
typically in an integral range. The plugin provides a mechanism to add new people
to the simulation and for other plugins to contribute to this creation process,
so that additional features of people can be specified at time of creation.
It also provides a mechanism for Components to provide Callbacks whenever new
people are created or when people are removed from a simulation. This mechanism
may release Events that indicate a person has been created or removed, and may include
events for stages of creation or removal, such as indicating that a person
will imminently be removed to enable Components or Plugins to access information about
a person prior to removal.

### Person Properties
This plugin defines typed key-value pairs for each person in the simulation.
The plugin also provides mechanisms to retrieve and set the current values of
properties for each person as well as for Components to provide Callbacks to
handle changes to these properties. This mechanism can also entail
releasing appropriate Events that indicate a person property has changed
and capturing its previous value.

### Regions
This plugin defines the concept of a region as an entity with a unique identifier,
generally intended to represent a geographic or conceptual location of a person
in a simulation. People in simulations with regions must be in exactly one region
at any point in time. Regions can have typed properties. This plugin provides
mechanisms to retrieve or set the region that a person is in as well as register
to handle when people move between regions. It similarly provides mechanisms to
retrieve or set region property values and to register handlers for these property
changes.

### Groups
This plugin defines the concept of a group that represents a relatively small
collection of people such as a home, a workplace, a school, a social group, etc.
Groups are typed and have unique ids with typed properties specific to the group
type. The plugin provides mechanisms to create or remove groups, add or remove
people from groups, obtain the members of a group, obtain the groups that a person
is in (by type), and to retrieve or set the properties for a given group. The
plugin also provides a mechanism to register to handle changes to group existence,
membership, or properties. People can be in zero to many groups of any given type.

### Partitions
This plugin provides a mechanism to track stratifications of the people in a
simulation by various characteristics, including person properties and region.
Partitions enable efficient lookup of the people associated with a given strata
as well as provide a mechanism for random sampling. A common use case of a partition
is enabling random selection of people in a given age group and geographic region
for contact processes. The plugin provides a mechanism to define new partitions
or remove existing partitions.

### Resources
This plugin defines typed resources that can be assigned to people and that are
in potentially limited supply. A resource could represent a durable or consumable
good - like a hospital bed (durable) or therapeutic dose (consumable). Resources
can have key-value properties defined on a type-specific basis. The plugin also
provides a mechanism to assign to, or remove a resource from, a person, change
the quantity of resources available in a simulation, and retrieve or set the value
of a resource property. The plugin also provides a mechanism for registering to
handle changes to any of these aspects of resource availability or property. Some
implementations of this plugin may assume the existence of the region plugin and
assume that resource availability is stratified by region.

### Random Number Generation
This plugin provides a mechanism to define independent random number generation
streams for the purposes of controlling random sampling processes in the simulation.
The plugin also provides a mechanism to seed/reseed the RNG streams from a common base.

### Reports
This plugin provides a convenience mechanism for adding Components or Plugins whose
purpose is to track information from an ongoing simulation for the purpose of producing
output reports. It also provides helper functions for collecting output from multiple
simulations and capturing simulation metadata that provide context for the parameter
values or other constructions used to define the simulation in question.
