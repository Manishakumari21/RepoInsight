import warnings

import numpy as np
import pandas as pd

from repoinsight_ml.compare import build_candidate_models
from repoinsight_ml.features import FEATURE_COLUMNS, rows_to_features
from repoinsight_ml.preprocessing import build_imputer
from repoinsight_ml.train import build_baseline_model


def _frame():
    # 23 columns: one partially missing, one completely missing.
    data = {col: [1.0, 2.0, 3.0] for col in FEATURE_COLUMNS}
    data["time_since_last_change_secs"] = [1.0, np.nan, 3.0]
    data["avg_change_delay_seconds"] = [np.nan, np.nan, np.nan]
    return pd.DataFrame(data, columns=FEATURE_COLUMNS)


def test_partially_missing_uses_median():
    X = _frame()
    out = build_imputer().fit_transform(X)
    idx = FEATURE_COLUMNS.index("time_since_last_change_secs")
    assert list(out[:, idx]) == [1.0, 2.0, 3.0]


def test_completely_missing_becomes_zero_and_columns_preserved():
    X = _frame()
    with warnings.catch_warnings():
        warnings.simplefilter("error")
        out = build_imputer().fit_transform(X)
    assert out.shape[1] == 23 == len(FEATURE_COLUMNS)
    idx = FEATURE_COLUMNS.index("avg_change_delay_seconds")
    assert list(out[:, idx]) == [0.0, 0.0, 0.0]
    assert not np.isnan(out).any()


def test_all_models_share_reusable_imputer_and_keep_23_features():
    X = _frame()
    y = pd.Series([0, 1, 0])
    for model in [
        build_baseline_model(),
        *build_candidate_models().values(),
    ]:
        with warnings.catch_warnings():
            warnings.simplefilter("error")
            model.fit(X, y)
        n = model.named_steps["imputer"].transform(X).shape[1]
        assert n == 23


def test_real_rows_keep_23_features_without_warning():
    from pathlib import Path

    from repoinsight_ml.dataset import load_jsonl

    dataset_path = Path(__file__).resolve().parents[1] / "data" / "dataset.jsonl"
    rows = load_jsonl(dataset_path)
    X, _ = rows_to_features(rows)
    assert list(X.columns) == FEATURE_COLUMNS
    with warnings.catch_warnings():
        warnings.simplefilter("error")
        out = build_imputer().fit_transform(X)
    assert out.shape[1] == 23
