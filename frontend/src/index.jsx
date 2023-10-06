import React from 'react';
import ReactDOM from "react-dom/client";
import App from './app';
import { ProvideState } from "./utils/state";
import { ProvideAuth } from "./utils/auth";

const rootElement = document.getElementById('root');
ReactDOM.createRoot(rootElement).render(
  <React.StrictMode>
    <ProvideAuth>
      <ProvideState>
        <App />
      </ProvideState>
    </ProvideAuth>
  </React.StrictMode>);
