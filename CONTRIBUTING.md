# Instructions for Logging Issues

## 1. Search for Duplicates

[Search the existing issues](https://github.com/pragmatismo-io/BotServer/issues) before logging a new one.

## 2. Do you have a question?

Please use the issue tracker for bugs and suggestions.
If you have a *question*, please use [Stack Overflow](https://stackoverflow.com/questions/tagged/botserver)

## 3. Did you find a bug?

We are not surprised, we're still in early preview so there are plenty of them right now.

When logging a bug, please be sure to include the following:
 * The platform you were using
 * If at all possible, an *isolated* way to reproduce the behavior
 * The behavior you expect to see, and the actual behavior

## 4. Do you have a suggestion?

We also accept suggestions in the issue tracker. 

In general, things we find useful when reviewing suggestions are:
* A description of the problem you're trying to solve
* An overview of the suggested solution
* Examples of how the suggestion would work in various places

# Instructions for Contributing Code

## Contributing bug fixes

General Bots is current in early preview. We're still accepting contributions in the form of bug fixes. 
A bug must have an issue tracking it in the issue tracker that has been approved by the pragmatismo.com.br team. Your pull request should include a link to the bug that you are fixing. If you've submitted a PR for a bug, please post a comment in the bug to avoid duplication of effort.

## Contributing features

Please open an issue with the `Schema` label to get a discussion started.

## Legal

We appreciate community contributions to code repositories open sourced by pragmatismo.com.br. By signing a contributor license agreement, we ensure that the community is free to use your contributions. 

## Housekeeping

Your pull request should: 

* Include a description of what your change intends to do
* Be a child commit of a reasonably recent commit in the **master** branch 
    * Requests need not be a single commit, but should be a linear sequence of commits (i.e. no merge commits in your PR)
* Have clear commit messages 
    * e.g. "Refactor feature", "Fix issue", "Add tests for issue"

##  You need to be able to run your system

from: http://catern.com/run.html

When developing a system, it is important to be able to run the system in its entirety.
"Run the unit tests" doesn't count. The complexity of your system is in the interactions between the units.

"Run an individual service against mocks" doesn't count. A mock will rarely behave identically to the real dependency, and the behavior of the individual service will be unrealistic. You need to run the actual system.

"Run an individual service in a shared stateful development environment running all the other services" doesn't count. A shared development environment will be unreliable as it diverges more and more from the real system.

"Run most services in a mostly-isolated development environment, calling out to a few hard-to-run external services" doesn't count. Those few external services on the edge of the mostly-isolated development environment are often the most crucial ones; without the ability to run modified versions of them, your development process is crippled. Furthermore, being dependent on external services greatly complicates where and how you can run the system; it's much harder to, for example, run tests with the system on every commit if that will access external services.

"Run all the services that make up the system in an isolated development environment" counts; it's the bare minimum requirement. Bonus points if this can be done completely on localhost, without using an off-host cluster deployment system.

Without the ability to actually run the entire system in this way while developing, many evil practices will tend to become common.

Testing is harder and far less representative, and therefore many issues can only be found when changes are deployed to production.
In turn, production deployment will cause issues more often, and so deployment will be more slow and less frequent.
Deploying the system to new environments is more difficult, since the developers aren't able to actually run the system. Existing practices in production will be cargo-culted and copied around indefinitely, even when they are unnecessary or actively harmful.
Exploratory usage of the system is very difficult, so it will be harder to consider using the system for purposes outside what it was originally developed for, and new use cases will become rare.
Downstream clients who depend on the system will also suffer all these issues, since without the ability to run the upstream system in development, they can't run their own entire system, which is a superset of the upstream system.
Running the entire system during development is the first step to preventing these issues. Further steps include writing automated tests for the system (which can be run repeatedly during development), and using, as much as possible, the same code to run the system in development and in production.
Developers of large or legacy systems that cannot already be run in their entirety during development often believe that it is impractical to run the entire system during development. They'll talk about the many dependencies of their system, how it requires careful configuration of a large number of hosts, or how it's too complex to get reliable behavior.

In my experience, they're always wrong. These systems can be run locally during development with a relatively small investment of effort. Typically, these systems are just ultimately not as complicated as people think they are; once the system's dependencies are actually known and understood rather than being cargo-culted or assumed, running the system, and all its dependencies, is straightforward.

Being able to run your entire system during development is just about the most basic requirement for a software project. It's not, on its own, sufficient for your development practices to be high quality; but if you can't do this, then you're not even in the running.


