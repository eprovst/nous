<div align="center">
  <img src="assets/logo.svg" width=210 />
  <p>Make your plain text knowledge base more useful.</p>
</div>

> [!WARNING]
> Νοῦς is still in early stages of development. Whilst the available commands
> should work, their names might change without warning or your data might end
> up corrupted!

## Installation
With a sufficiently recent version of Rust installed run `cargo install
--path=.` from the repository root.

To enable shell completions add `source <(COMPLETE=bash nous)` to your `.bashrc`
or equivalent. See the [documentation of the completion
library](https://docs.rs/clap_complete/latest/clap_complete/env/index.html) for
more details about supported shells.

## Usage
Run `nous help` for a list of the available commands. To work with nodes within
a νοῦς realm, it will first have to be initialized (which, for now, comes down
to making a `.nous` folder at the root of the realm). You can initialize a realm
in the current working directory using `nous init` or `nous init
path/to/intended/root`.

Once a realm is initialized, other commands (if enabled, with helpful tab
completion) can be applied on node names, no matter where they live within the
realm. In particular you can search for incoming and outgoing wikilinks
(i.e. links of the form `[[other node]]`), which are called backlinks and
forward links, respectively. For further detail look at the output of `nous
help`.

## What is 'νοῦς'?
Nous (from the Greek 'νοῦς') is a concept found in classical philosophy which
refers to the part of the human mind necessary to understand what is true and
real, making it (in the author's opinion) a fitting name for a knowledge
management tool. If the philosophical details interest you, the [Stanford
Encyclopedia of
Philosophy](https://plato.stanford.edu/entries/aristotle-ethics/#Glos) gives a
good introduction to the various interpretations of this notion.
