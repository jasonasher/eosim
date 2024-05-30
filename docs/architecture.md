# Eosim Architecture

Eosim is a discrete event-based framework designed to allow for the
creation of large-scale simulations. The primary use case is
construction of agent-based models for disease transmission, but the
approach is applicable in a wide array of circumstances.


# Basic Concepts

## Overall Architecture

<!-- Need a new term for module -->

The central object of an Eosim simulation is the `Context`, which is
responsible for managing all the behavior of the simulation. 
All of the simulation specific logic is embedded in modules
which rely on the `Context` for core services such as:

* Maintaining a notion of time for the simulation.
* Scheduling events to occur at some point in the future
  and executing them at that time.
* Holding module specific data so that the module
  and other modules can access it
  
In practice, a simulation usually consists of a set of modules
which work together to provide all the functions of the simulation.
For instance, a simple SIR model might consist of the following
modules:

* A population loader which initializes the set of people represented
  by the simulation.
* An infection seeder which introduces the pathogen into the population.
* A transmission manager which models the process of an infected
  person trying to infect a susceptible person.

Eosim ships with a basic set of modules that provide commonly
used facilities, such as:

* `person_properties` which models the concept of a person and
  and allows other modules to attach properties (e.g.,
  "is infected").
  
* `regions` which models the concept of a physical location
  (e.g., a state) and the people in it.
  
* `random` which provides a deterministic pseudorandom number generator.

While these modules are built-in, they are not privileged;
for instance, it is possible to write your own random number
generator. However as a practical matter it is best to use the
built-ins when possible.


## Time Keeping and Actions

A simulation in Eosim is essentially just an event loop which runs
time forward and executes actions at their appointed time. Modules
perform their roles in the simulation by registering to perform
actions. When there are no more actions to be executed, the simulation
terminates.

There are two major approaches to handling time in simulations:

* Move time forward in constant increments and at each increment
  poll each actor for whether it wants to act in that time step.
  
* Allow actors to register to perform actions at specific times
  in the future and then execute those actions in order while
  skipping over intermediate time steps.
  
Eosim uses the second approach, which we have found to be more
efficient in several respects.

1. It allows the simulation to skip over intermediate time steps
   during which nothing is happening. This is a common situation
   in simulations where events (e.g., pathogen transmission)
   are comparatively rare.
   
1. It permits actions to be scheduled at arbitrarily small time
   resolutions (e.g., fractions of days or even seconds) without
   performance penalty.

It is also possible to schedule actions to happen immediately
(which is essentially equivalent to an infinitely small time
in the future). This can be used to execute actions before
the next time step without creating reentrancy (e.g., if
module **A** is listening to be notified of an action by module **B**
and **A** responds by taking an action which **B** wants to be
notified of).


## Data Storage

Many modules will want to store data. For instance, a SIR model
simulation needs to store the S/I/R state of each person in the
simulation. Naively, the way to do this would be to store the
data in the module itself, but this has several drawbacks
in practice:

* It makes it hard to share data between modules because then
  you have to write accessors.
  
* It creates issues with the Rust data ownership model.

* [TODO: others?]

In Eosim, modules store data in *data containers* which are attached
to the `Context` rather than a module object (indeed, modules don't
typically have any attached data). A data container can be any Rust
type but is typically some kind of collection object.

As an example, if you were implementing an SIR model, you might
have a population of size *N* with identifiers being integers
in the range *[1..N]*. The model would have an `InfectionState` data
container which is a map of`PersonId` &rarr;
`InfectionState`. The transmission manager would simulate a contact
as follows:

* Draw a random integer *R* in the range *[1..N]*, excluding the
  infected person.
* Retrieve the `InfectionState` data container from the context.
* Check the infection state of `InfectionState[R]` and if
  its `Susceptible`, infect them.







