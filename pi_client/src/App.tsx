import React from 'react';
import { BrowserRouter as Router } from 'react-router-dom';

import Main from './Main';
import Nav from './Nav';
import { AppContextProvider } from './State';

export default function App() {
  return (
    <div className="content d-flex flex-row flex-nowrap align-items-stretch">
      <AppContextProvider>
        <Router>
          <Nav />
          <Main />
        </Router>
      </AppContextProvider>
    </div>
  );
}