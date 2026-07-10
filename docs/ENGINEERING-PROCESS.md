# Jellyx Engineering Process

Jellyx is an AI-assisted software project, but it is human-directed, human-reviewed, and human-maintained.

This document exists to explain the workflow plainly, not to make AI the center of the product.

## Short version

- The project direction, requirements, architecture, and acceptance criteria are human-led.
- AI is used to help with parts of implementation and documentation.
- Every meaningful change is still reviewed, constrained, and validated by a human.
- The goal is reviewable software delivery, not autonomous shipping.

## What that means in practice

| Area | How Jellyx handles it |
|---|---|
| Product direction | Human-defined |
| Requirements | Human-written and iterated |
| Architecture | Human-led |
| Implementation | AI-assisted |
| Verification | Human-reviewed with rigid checks |
| Acceptance | Human decision |

## Process principles

1. **Human in the loop at all times**
   The project is continuously directed by a human. AI does not choose goals, scope, or release decisions on its own.

2. **Requirements before implementation**
   Changes are driven by explicit requirements, not by improvising code first and rationalizing later.

3. **Deterministic validation**
   Work is checked through reproducible build, test, and review steps whenever possible.

4. **Constrained use of AI**
   AI output is bounded by review scope, conventions, acceptance criteria, and engineering judgment.

5. **Engineering accountability stays human**
   If something is released, the human maintainer owns the decision, the constraints, and the review of that change.

## What Jellyx is not

Jellyx is not presented as:

- fully hand-written software
- autonomous AI software
- unreviewed AI output
- "just prompt it and ship it" development

## Why document this

The repo should be honest about how the software is produced.

That means:

- not pretending the code is entirely hand-written when it is not
- not outsourcing product, architecture, or release accountability to a tool

If you are evaluating the project, judge it by the quality of its requirements, architecture, review discipline, and runtime behavior.
