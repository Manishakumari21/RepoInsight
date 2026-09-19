import joblib
import numpy as np
import pandas as pd
import pytest

from repoinsight_ml.compare import build_candidate_models
from repoinsight_ml.features import FEATURE_COLUMNS
from repoinsight_ml.model_persistence import (
    build_metadata,
    load_model,
    load_model_with_metadata,
    persist_candidate_models,
    save_model,
)


def _frame(n=12):
    rng = np.random.RandomState(0)
    data = {c: rng.uniform(0, 10, size=n) for c in FEATURE_COLUMNS}
    # Keep one partially-missing and one fully-missing column pattern.
    data["time_since_last_change_secs"] = [1.0 if i % 2 else np.nan for i in range(n)]
    data["avg_change_delay_seconds"] = [np.nan] * n
    X = pd.DataFrame(data, columns=FEATURE_COLUMNS)
    y = pd.Series([i % 2 for i in range(n)], dtype="int64")
    return X, y


def _fitted_models():
    X, y = _frame()
    models = build_candidate_models()
    for m in models.values():
        m.fit(X, y)
    return models, X, y


def test_save_load_roundtrip_probabilities_match(tmp_path):
    models, X, _ = _fitted_models()
    for name, model in models.items():
        path = tmp_path / f"{name}.joblib"
        save_model(model, path, build_metadata(name, model, len(X)))
        loaded = load_model(path)
        assert np.array_equal(loaded.predict(X), model.predict(X))
        assert np.allclose(
            loaded.predict_proba(X), model.predict_proba(X), atol=1e-12
        )


def test_complete_pipeline_persisted_and_usable(tmp_path):
    models, X, y = _fitted_models()
    for name, model in models.items():
        path = tmp_path / f"{name}.joblib"
        save_model(model, path, build_metadata(name, model, len(X)))
        loaded, meta = load_model_with_metadata(path)
        assert "imputer" in loaded.named_steps
        assert "classifier" in loaded.named_steps
        if name == "logistic_regression":
            assert "scaler" in loaded.named_steps
        assert meta["feature_count"] == 23
        assert meta["feature_list"] == FEATURE_COLUMNS
        assert meta["model_name"] == name
        assert "sklearn_version" in meta and "train_rows" in meta
        assert "token" not in str(meta).lower()
        # Usable on new data with same schema.
        assert len(loaded.predict(X)) == len(y)


def test_feature_mismatch_fails_clearly(tmp_path):
    models, X, _ = _fitted_models()
    model = models["random_forest"]
    path = tmp_path / "bad.joblib"
    meta = build_metadata("random_forest", model, len(X))
    meta["feature_list"] = ["wrong_feature"]
    meta["feature_count"] = 1
    joblib.dump({"model": model, "metadata": meta}, path)
    with pytest.raises(ValueError, match="Feature schema mismatch"):
        load_model(path)


def test_missing_and_invalid_artifacts_raise(tmp_path):
    with pytest.raises(FileNotFoundError):
        load_model(tmp_path / "does_not_exist.joblib")
    broken = tmp_path / "broken.joblib"
    broken.write_bytes(b"not a joblib file")
    with pytest.raises(ValueError, match="Invalid model artifact"):
        load_model(broken)


def test_persist_all_writes_expected_artifacts(tmp_path):
    models, X, _ = _fitted_models()
    payload = {n: (m, len(X)) for n, m in models.items()}
    paths = persist_candidate_models(payload, tmp_path / "artifacts")
    assert set(paths) == {
        "logistic_regression",
        "random_forest",
        "hist_gradient_boosting",
    }
    for p in paths.values():
        assert p.is_file()
    assert (tmp_path / "artifacts" / "metadata.json").is_file()
    # Deterministic: reload gives identical probabilities.
    X2, _ = _frame()
    for name, p in paths.items():
        assert np.allclose(
            load_model(p).predict_proba(X2),
            models[name].predict_proba(X2),
            atol=1e-12,
        )
