"""Tests for v2 result extraction."""

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent.parent))

from extract_results import build_results_skeleton


class TestResultsSkeleton:
    def test_skeleton_has_required_keys(self) -> None:
        skel = build_results_skeleton("wmdp_chem", "Qwen/Qwen2.5-7B-Instruct")
        assert skel["eval"] == "wmdp_chem"
        assert skel["model"] == "Qwen/Qwen2.5-7B-Instruct"
        assert skel["cells"] == []
        assert skel["item_matrix"] == {}

    def test_skeleton_model_preserved(self) -> None:
        skel = build_results_skeleton("wmdp_bio", "Qwen/Qwen2.5-72B-Instruct")
        assert skel["model"] == "Qwen/Qwen2.5-72B-Instruct"
