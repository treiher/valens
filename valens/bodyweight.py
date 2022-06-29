import pandas as pd


def avg_weight(df: pd.DataFrame) -> pd.Series:
    return df.rolling(window=9, center=True).mean()["weight"]
