import React from 'react'
import ReactDOM from 'react-dom/client'
import App from './App.jsx'
import 'leaflet/dist/leaflet.css';
// Если у вас был импорт CSS (например, import './index.css'), добавьте его сюда

if (import.meta.env.DEV) {
  import('./dev/probeEvalBaselineEntrypoint')
    .then((mod) => mod.registerProbeEvalBaselineEntrypoint())
    .catch(() => {});
}

ReactDOM.createRoot(document.getElementById('root')).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
)
