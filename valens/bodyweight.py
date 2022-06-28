import pandas as pd


def avg_weight(df: pd.DataFrame) -> pd.Series:
    return df.rolling(window=9, center=True).mean()["weight"]


def avg_weight_change(df: pd.DataFrame) -> pd.Series:
    assert df.index.name == "date"
    ts_df = pd.DataFrame({"date": pd.date_range(df.index[0], df.index[-1])}).set_index("date")
    return df.join(
        ts_df.join(df["avg_weight"]).interpolate().pct_change(periods=7, fill_method=None).mul(100),
        rsuffix="_change",
    )["avg_weight_change"].iloc[0:-4]
