import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import App from './App';
import './index.css';

const container = document.getElementById('root');
if (!container) throw new Error('#root is missing from index.html');

createRoot(container).render(
  <StrictMode>
    <App />
  </StrictMode>,
);

// The webview's own context menu and drag-navigation are not useful in an
// application window, and dropping a file outside the composer would
// otherwise navigate away from the app entirely.
if (!import.meta.env.DEV) {
  document.addEventListener('contextmenu', (e) => e.preventDefault());
}
window.addEventListener('dragover', (e) => e.preventDefault());
window.addEventListener('drop', (e) => e.preventDefault());
