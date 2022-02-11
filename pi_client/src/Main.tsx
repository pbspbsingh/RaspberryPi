import React from 'react';
import { Route, Routes } from 'react-router-dom';
import { useDataFetcher } from './dataFetcher';

import Footer from './Footer';
import Config from './pages/Config';
import Dashboard from './pages/Dashboard';
import Health from './pages/Health';
import Queries from './pages/Queries';

export default function Main(): JSX.Element {
    useDataFetcher();
    return (
        <main className="d-flex flex-column">
            <div className="main-content container-fluid flex-grow-1">
                <Routes>
                    <Route path="/queries" element={<Queries />} />
                    <Route path="/filters" element={<Config />} />
                    <Route path="/health" element={<Health />} />
                    <Route path="/" element={<Dashboard />} />
                </Routes>
            </div>
            <Footer />
        </main>
    );
}