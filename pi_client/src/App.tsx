import React from 'react';
import { BrowserRouter as Router } from 'react-router-dom';

import Main from './Main';
import Nav from './Nav';

export default function App() {
  return (
    <div className="content d-flex flex-row flex-nowrap align-items-stretch">
      <Router>
        <Nav />
        <Main />
      </Router>
    </div>
  );
}