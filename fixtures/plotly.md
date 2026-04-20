# Plotly diagrams

## Bar chart

```plotly
{
  "data": [
    {
      "type": "bar",
      "x": ["parse", "transform", "render"],
      "y": [12, 4, 31],
      "marker": { "color": "#6c8ebf" }
    }
  ],
  "layout": {
    "title": "mdview pipeline costs (ms)",
    "xaxis": { "title": "stage" },
    "yaxis": { "title": "ms" }
  }
}
```

## Line chart

```plotly
{
  "data": [
    {
      "type": "scatter",
      "mode": "lines+markers",
      "x": [0, 1, 2, 3, 4, 5, 6, 7, 8, 9],
      "y": [0, 1, 4, 9, 16, 25, 36, 49, 64, 81],
      "name": "y = x^2"
    }
  ],
  "layout": {
    "title": "Quadratic growth",
    "xaxis": { "title": "x" },
    "yaxis": { "title": "y" }
  }
}
```

## Pie chart

```plotly
{
  "data": [
    {
      "type": "pie",
      "labels": ["Rust", "TypeScript", "Lua"],
      "values": [70, 25, 5],
      "hole": 0.4
    }
  ],
  "layout": { "title": "mdview language mix" }
}
```

## Scatter (3D)

```plotly
{
  "data": [
    {
      "type": "scatter3d",
      "mode": "markers",
      "x": [1, 2, 3, 4, 5],
      "y": [5, 4, 3, 2, 1],
      "z": [2, 3, 5, 7, 11],
      "marker": { "size": 6 }
    }
  ],
  "layout": { "title": "3D scatter" }
}
```
