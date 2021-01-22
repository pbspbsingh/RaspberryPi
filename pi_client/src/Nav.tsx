import React from 'react';
import { Link, useLocation } from 'react-router-dom';

import { ClipBoard, Gear, SpeedoMeter, ThermoMeter } from './Icons';

export default function Nav(): JSX.Element {
    const { pathname } = useLocation();
    return (
        <nav className="d-flex flex-column align-items-center">
            <Link to="/" className="logo"><img src="android-chrome-192x192.png" alt="logo" /></Link>
            <ul className="navigation list-unstyled d-flex flex-column">
                <li className={makeActive(pathname, '')}>
                    <Link to="/"><SpeedoMeter />Dashboard</Link>
                </li>
                <li className={makeActive(pathname, 'queries')}>
                    <Link to="/queries"><ClipBoard />Queries</Link>
                </li>
                <li className={makeActive(pathname, 'config')}>
                    <Link to="/config"><Gear />Config</Link>
                </li>
                <li className={makeActive(pathname, 'health')}>
                    <Link to="/health"><ThermoMeter />Health</Link>
                </li>
            </ul>
        </nav>
    );
}

function makeActive(pathName: string, current: string): string {
    if (pathName.startsWith("/")) {
        pathName = pathName.substring(1);
    }
    if (pathName.trim() === current) {
        return "active";
    }
    else {
        return "";
    }
}