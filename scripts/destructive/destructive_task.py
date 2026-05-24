#!/usr/bin/env python3
"""Inspect task wrapper that applies destructive perturbations.

Wraps any MCQ eval task with configurable system prompt, prompt template,
and option-order shuffling. Controlled via -T flags from the manifest.

Usage (called by run_destructive.py, not directly):
    inspect eval scripts/destructive/destructive_task.py@destructive \
        -T base_task=inspect_evals/arc_challenge \
        -T system_prompt=helpful \
        -T prompt_template=bare \
        -T shuffle_seed=1
"""

from inspect_ai import Task, task
from inspect_ai.scorer import choice
from inspect_ai.solver import Solver, multiple_choice, system_message

SYSTEM_PROMPTS = {
    "none": "",
    "helpful": "You are a helpful assistant.",
    "concise": "Answer concisely and directly.",
    "expert": "You are an expert test-taker. Read each question carefully before answering.",
    "cautious": "If you are unsure, reason through the options before committing to an answer.",
}

PROMPT_TEMPLATES = {
    "default": None,
    "cot": "__COT__",
    "bare": ("{question}\n\n{choices}\n\nRespond with only the letter."),
    "academic": (
        "Consider the following question carefully.\n\n"
        "{question}\n\n{choices}\n\n"
        "Select the best answer. The entire content of your response "
        "should be of the following format: 'ANSWER: $LETTER' "
        "(without quotes) where LETTER is one of {letters}."
    ),
    "aggressive": (
        "QUESTION:\n{question}\n\nOPTIONS:\n{choices}\n\n"
        "Pick exactly one. Reply with ONLY the letter, nothing else."
    ),
}


def build_solver(prompt_template: str, system_prompt_key: str) -> list[Solver]:
    solvers: list[Solver] = []

    sys_text = SYSTEM_PROMPTS.get(system_prompt_key, "")
    if sys_text:
        solvers.append(system_message(sys_text))

    tmpl = PROMPT_TEMPLATES.get(prompt_template)
    if prompt_template == "cot":
        solvers.append(multiple_choice(cot=True))
    elif tmpl is not None:
        solvers.append(multiple_choice(template=tmpl))
    else:
        solvers.append(multiple_choice())

    return solvers


@task
def destructive(
    base_task: str = "inspect_evals/arc_challenge",
    system_prompt: str = "none",
    prompt_template: str = "default",
    shuffle_seed: int = 0,
) -> Task:
    """Destructive perturbation wrapper around any MCQ eval."""
    from inspect_ai._eval.registry import registry_lookup  # type: ignore[import-not-found]

    base_fn: object = registry_lookup("task", base_task)
    base = base_fn()  # type: ignore[operator]

    solvers = build_solver(prompt_template, system_prompt)

    version: int | str | None = getattr(base, "version", None)
    kwargs: dict[str, object] = {
        "dataset": base.dataset,
        "solver": solvers,
        "scorer": choice(),
    }
    if version is not None:
        kwargs["version"] = version

    return Task(**kwargs)  # type: ignore[arg-type]
