from __future__ import annotations

from sklearn.impute import SimpleImputer
from sklearn.pipeline import Pipeline


def build_imputer() -> Pipeline:
    """Median imputation with deterministic zero fallback.

    Keeps all 23 feature columns: partially observed columns use the
    median, while columns with no observed values (e.g. legitimate
    null Option fields like avg_change_delay_seconds) fall back to 0
    instead of being dropped with a warning.
    """
    return Pipeline(
        [
            (
                "median",
                SimpleImputer(strategy="median", keep_empty_features=True),
            ),
            (
                "fallback",
                SimpleImputer(strategy="constant", fill_value=0),
            ),
        ]
    )
