# AI Game

_title work in progress_

_This repository is mirrored at [GitHub][github-repo]_

This is an experimental text-based adventure game that uses a
locally-hosted large language model (LLM) to:

* Create the game world
* Parse commands
* Determine the result of executing commands.

The purpose of this game is to learn more about large language models
and the challenges involved with working with them, learn new
technologies, and update my knowledge on Rust and NoSQL solutions. And
of course, also play a game.

This codebase is currently under heavy work, and the game is not
really playable. Exploration of the world in a (mostly?) coherent
manner is possible. Running the application requires a bunch of manual
setup, which I aim to eventually resolve.

## Differences Compared to Other AI-driven RPGs/Fiction Software

Unlike existing AI-based roleplaying/fiction writing software, this
game aims to keep as much state out of the LLM as possible. The LLM is
used as a creative driving force to develop the world, while
interacting with the game world is still mostly handled through
programmed logic like a traditional text-based adventure.

# General Roadmap

A very high level of things I want to accomplish.

* Implementation of all events to allow for basic gameplay.
* Addition of a game mechanics system (levels, classes, combat, etc).
* Bundle the application and all its dependencies in a one-click
  executable.
* Swap out Kobold API for OpenAI API, to allow for greater
  compatibility with existing LLM solutions.
* Make a GUI (low priority).

# Technical Documentation

Short technical documentation. To be improved.

## Technologies/Dependencies

These technologies are used:

* [Rust][rust-lang]
* [ArangoDB][arangodb]
* [KoboldCPP][koboldcpp]
* [Mistral 7b Instruct Model][ai-model]

## Building and Running

The application is built using `cargo`, the Rust build tool and
dependency manager. Running the application requires running instances
of both ArangoDB and KoboldCPP. See the websites of these projects for
how to run them. Additionally, this application currently expects
Kobold to use the Mistral 7B Instruct model to generate content. Using
other models will possibly lead to unexpected or incoherent results.

Better instructions will follow as the application becomes more
usable.

## How Does It Work?

The game loop is based on a fairly simple concept: enter command, ask
LLM to "parse" command, then ask the LLM to "execute" the command. The
game is being built on an architecture similar to the event sourcing
pattern: the LLM generates a list of events that apply to the player,
NPCs, or game world.

The entire game world is stored in the ArangoDB graph database. The
eventual aim is to store generated events and a mutable copy of the
world in the database alongside the unaltered version, so that the
game state can be recreated, restarted, and saved easily.

Most of the complexity in the code revolves around making sure that
the information coming from the LLM is coherent. While the Mistral 7b
Instruct model is very good at following instructions, it sometimes
generates nonsensical data.

# License

<img src="./agplv3.png" alt="AGPLv3" />

The game is licensed under the [AGPLv3][agpl]. The game is free
software, that you can run, redistribute, modify, study, and learn
from as you see fit, as long as you extend that same freedom to
others.

[rust-lang]: https://www.rust-lang.org/
[arangodb]: https://arangodb.com/
[koboldcpp]: https://github.com/LostRuins/koboldcpp
[ai-model]: https://huggingface.co/TheBloke/Mistral-7B-Instruct-v0.2-GGUF
[agpl]: https://www.gnu.org/licenses/agpl-3.0.en.html
[github-repo]: https://github.com/ProjectMoon/ai-game
