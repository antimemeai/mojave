"""Tests for v2 MCQ task wrapper."""

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent.parent))

from mcq_task import (
    BASE_TEMPLATES,
    SYSTEM_PROMPTS,
    build_solver_chain,
    build_template,
)


class TestBaseTemplates:
    def test_five_templates_defined(self) -> None:
        assert len(BASE_TEMPLATES) == 5

    def test_template_names_match_spec(self) -> None:
        expected = {
            "lm-eval-default",
            "bare",
            "cot",
            "letter-only",
            "verbose-rationale",
        }
        assert set(BASE_TEMPLATES.keys()) == expected

    def test_lm_eval_default_has_answer_format(self) -> None:
        tmpl = BASE_TEMPLATES["lm-eval-default"]
        assert "ANSWER: $LETTER" in tmpl

    def test_bare_is_minimal(self) -> None:
        tmpl = BASE_TEMPLATES["bare"]
        assert tmpl == "{question}\n\n{choices}"

    def test_cot_has_think_step_by_step(self) -> None:
        tmpl = BASE_TEMPLATES["cot"]
        assert "Think step by step" in tmpl

    def test_all_templates_have_question_placeholder(self) -> None:
        for name, tmpl in BASE_TEMPLATES.items():
            assert "{question}" in tmpl, f"{name} missing {{question}}"

    def test_all_templates_have_choices_placeholder(self) -> None:
        for name, tmpl in BASE_TEMPLATES.items():
            assert "{choices}" in tmpl, f"{name} missing {{choices}}"


class TestSystemPrompts:
    def test_four_prompts_defined(self) -> None:
        assert len(SYSTEM_PROMPTS) == 4

    def test_prompt_names_match_spec(self) -> None:
        expected = {"none", "helpful", "domain-expert", "safety-aware"}
        assert set(SYSTEM_PROMPTS.keys()) == expected

    def test_none_is_empty(self) -> None:
        assert SYSTEM_PROMPTS["none"] == ""


class TestBuildTemplate:
    def test_no_prefix_returns_base(self) -> None:
        tmpl = build_template("bare", "")
        assert tmpl == "{question}\n\n{choices}"

    def test_prefix_prepended(self) -> None:
        prefix = "Example:\nQ?\nA) x\nANSWER: A\n\n"
        tmpl = build_template("bare", prefix)
        assert tmpl.startswith("Example:")
        assert tmpl.endswith("{question}\n\n{choices}")

    def test_all_templates_accept_prefix(self) -> None:
        prefix = "EX\n\n"
        for name in BASE_TEMPLATES:
            tmpl = build_template(name, prefix)
            assert tmpl.startswith("EX\n\n")
            assert "{question}" in tmpl
            assert "{choices}" in tmpl


class TestSolverChain:
    def test_no_system_message_when_none(self) -> None:
        solvers = build_solver_chain(
            prompt_template="bare",
            system_prompt="none",
            nshot_prefix="",
        )
        assert len(solvers) == 1

    def test_system_message_added_when_present(self) -> None:
        solvers = build_solver_chain(
            prompt_template="bare",
            system_prompt="helpful",
            nshot_prefix="",
        )
        assert len(solvers) == 2

    def test_solver_chain_returns_list(self) -> None:
        solvers = build_solver_chain(
            prompt_template="lm-eval-default",
            system_prompt="none",
            nshot_prefix="prefix\n\n",
        )
        assert isinstance(solvers, list)
        assert len(solvers) >= 1
