import {
  Chart as ChartJS,
  RadialLinearScale,
  PointElement,
  LineElement,
  Filler,
  Tooltip,
  Legend,
} from "chart.js";
import { Radar } from "react-chartjs-2";
import { CategoryKey } from "../../lib/api/types";
import { categoryLabelMap } from "../../lib/categories";

ChartJS.register(RadialLinearScale, PointElement, LineElement, Filler, Tooltip, Legend);

interface RadarChartProps {
  scores: Record<CategoryKey, number>;
  title?: string;
}

function RadarChart({ scores, title }: RadarChartProps) {
  const labels = Object.keys(scores).map((k) => categoryLabelMap[k as CategoryKey]);
  const dataValues = Object.values(scores);
  const data = {
    labels,
    datasets: [
      {
        label: title ?? "scores",
        data: dataValues,
        backgroundColor: "rgba(99, 102, 241, 0.2)",
        borderColor: "rgba(99, 102, 241, 1)",
        borderWidth: 2,
        pointBackgroundColor: "rgba(99, 102, 241, 1)",
      },
    ],
  };

  const options = {
    responsive: true,
    scales: {
      r: {
        beginAtZero: true,
        suggestedMax: 1,
        ticks: {
          stepSize: 0.2,
        },
      },
    },
    plugins: {
      legend: { display: false },
    },
  };

  return <Radar data={data} options={options} />;
}

export default RadarChart;
