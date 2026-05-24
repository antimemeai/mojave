"""Tests for n-shot exemplar pool management."""

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent.parent))

from inspect_ai.dataset import Sample
from nshot import (
    EXEMPLAR_SEED,
    N_SHOT_FRACS,
    build_nshot_prefix,
    compute_n_for_frac,
    draw_exemplar_pool,
    format_exemplar,
    serialize_pool,
)


def make_samples(n: int) -> list[Sample]:
    return [
        Sample(
            input=f"Question {i}?",
            choices=[f"A{i}", f"B{i}", f"C{i}", f"D{i}"],
            target="B",
            id=f"q{i:04d}",
        )
        for i in range(n)
    ]


class TestDrawPool:
    def test_pool_size_matches_fraction(self) -> None:
        samples = make_samples(400)
        pool = draw_exemplar_pool(samples, max_frac=0.05)
        assert len(pool) == 20

    def test_pool_is_deterministic(self) -> None:
        samples = make_samples(400)
        a = draw_exemplar_pool(samples, max_frac=0.05)
        b = draw_exemplar_pool(samples, max_frac=0.05)
        assert [s.id for s in a] == [s.id for s in b]

    def test_pool_items_are_from_dataset(self) -> None:
        samples = make_samples(100)
        pool = draw_exemplar_pool(samples, max_frac=0.05)
        sample_ids = {s.id for s in samples}
        for s in pool:
            assert s.id in sample_ids

    def test_no_duplicates_in_pool(self) -> None:
        samples = make_samples(400)
        pool = draw_exemplar_pool(samples, max_frac=0.05)
        ids = [s.id for s in pool]
        assert len(ids) == len(set(ids))


class TestComputeN:
    def test_zero_frac(self) -> None:
        assert compute_n_for_frac(0.0, 400) == 0

    def test_one_percent(self) -> None:
        assert compute_n_for_frac(0.01, 400) == 4

    def test_five_percent(self) -> None:
        assert compute_n_for_frac(0.05, 400) == 20

    def test_minimum_one_for_nonzero_frac(self) -> None:
        assert compute_n_for_frac(0.01, 50) == 1

    def test_wmdp_chem_levels(self) -> None:
        assert compute_n_for_frac(0.01, 408) == 4
        assert compute_n_for_frac(0.025, 408) == 10
        assert compute_n_for_frac(0.05, 408) == 20

    def test_wmdp_bio_levels(self) -> None:
        assert compute_n_for_frac(0.01, 1273) == 12
        assert compute_n_for_frac(0.025, 1273) == 31
        assert compute_n_for_frac(0.05, 1273) == 63


class TestFormatExemplar:
    def test_format_includes_question(self) -> None:
        s = Sample(input="What is X?", choices=["a", "b", "c", "d"], target="C", id="q1")
        text = format_exemplar(s)
        assert "What is X?" in text

    def test_format_includes_choices(self) -> None:
        s = Sample(
            input="Q?",
            choices=["alpha", "beta", "gamma", "delta"],
            target="A",
            id="q1",
        )
        text = format_exemplar(s)
        assert "A) alpha" in text
        assert "B) beta" in text
        assert "C) gamma" in text
        assert "D) delta" in text

    def test_format_includes_answer(self) -> None:
        s = Sample(input="Q?", choices=["a", "b", "c", "d"], target="B", id="q1")
        text = format_exemplar(s)
        assert "ANSWER: B" in text


class TestBuildPrefix:
    def test_zero_shot_returns_empty(self) -> None:
        pool = make_samples(10)
        assert build_nshot_prefix(pool, 0) == ""

    def test_nshot_includes_n_exemplars(self) -> None:
        pool = make_samples(10)
        prefix = build_nshot_prefix(pool, 3)
        assert "Question 0?" in prefix
        assert "Question 1?" in prefix
        assert "Question 2?" in prefix
        assert "Question 3?" not in prefix

    def test_prefix_is_nested(self) -> None:
        pool = make_samples(10)
        prefix_2 = build_nshot_prefix(pool, 2)
        prefix_5 = build_nshot_prefix(pool, 5)
        assert prefix_5.startswith(prefix_2.rstrip("\n"))

    def test_prefix_ends_with_separator(self) -> None:
        pool = make_samples(10)
        prefix = build_nshot_prefix(pool, 3)
        assert prefix.endswith("\n\n")


class TestSerializePool:
    def test_serialization_has_seed(self) -> None:
        pool = make_samples(20)
        data = serialize_pool(pool, dataset_size=400)
        assert data["seed"] == EXEMPLAR_SEED

    def test_serialization_has_all_frac_levels(self) -> None:
        pool = make_samples(20)
        data = serialize_pool(pool, dataset_size=400)
        for frac in N_SHOT_FRACS:
            assert str(frac) in data["levels"]

    def test_serialization_has_hash(self) -> None:
        pool = make_samples(20)
        data = serialize_pool(pool, dataset_size=400)
        assert len(data["pool_sha256"]) == 64

    def test_serialization_deterministic(self) -> None:
        pool = make_samples(20)
        a = serialize_pool(pool, dataset_size=400)
        b = serialize_pool(pool, dataset_size=400)
        assert a == b

    def test_level_nesting(self) -> None:
        pool = make_samples(20)
        data = serialize_pool(pool, dataset_size=400)
        ids_1pct = data["levels"]["0.01"]["item_ids"]
        ids_5pct = data["levels"]["0.05"]["item_ids"]
        assert all(i in ids_5pct for i in ids_1pct)
