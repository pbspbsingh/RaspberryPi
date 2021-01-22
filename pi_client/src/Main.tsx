import React from 'react';
import { Route, Switch } from 'react-router-dom';

import Footer from './Footer';
import Config from './pages/Config';
import Dashboard from './pages/Dashboard';
import Health from './pages/Health';
import Queries from './pages/Queries';

export default function Main(): JSX.Element {
    return (
        <main className="d-flex flex-column">
            <div className="main-content container-fluid">
                <Switch>
                    <Route path="/queries">
                        <Queries />
                    </Route>
                    <Route path="/config">
                        <Config />
                    </Route>
                    <Route path="/health">
                        <Health />
                    </Route>
                    <Route path="/">
                        <Dashboard />
                    </Route>
                </Switch>
            </div>
            <Footer />
        </main>
    );
}