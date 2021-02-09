import React, { useContext, useEffect } from 'react';
import ReactApexChart from 'react-apexcharts';

import { Loader } from '../Icons';
import { AppContext, DATE_RANGE } from '../State';

import { loadDashboard } from '../dataFetcher';

const REFRESH_TIMEOUT = 1 * 60 * 1000;

export default function Dashboard(): JSX.Element {
    const { state, dispatch } = useContext(AppContext);
    const { status, clickedDays, errorMsg, dashLastUpdated, dashboardData, days } = state;

    useEffect(() => {
        if (days !== clickedDays || status !== "DONE" || Date.now() - dashLastUpdated > REFRESH_TIMEOUT) {
            dispatch({ type: "SET_LOADING" });
            loadDashboard(dispatch, clickedDays);
        }
        const refresher = setInterval(() => loadDashboard(dispatch, clickedDays), REFRESH_TIMEOUT);
        return () => clearInterval(refresher);
    }, [clickedDays]);

    const dnsRequestsData = dashboardData?.dns_data ?? [];
    const msData = dashboardData?.latency_data ?? [];

    const datesLength = Object.keys(DATE_RANGE).length;
    return (
        <section className="h-100">
            <header>
                <div className="d-flex align-items-center">
                    <h2>Dashboard</h2> &nbsp;
                    {dashboardData != null && <p><b>Blocked: </b>{dashboardData.reject_count}/{dashboardData.total_count}
                    ({(100 * dashboardData.reject_count / dashboardData.total_count).toFixed(2)}%)</p>}
                </div>
                <p className="filter-date-range">
                    Date Range:
                    {Object.entries(DATE_RANGE).map(([currDays, name], idx) =>
                    <span key={idx}>
                        <a href="#" className={days === currDays ? "selected" : ""}
                            onClick={() => { dispatch({ type: "CLICKED_DAYS", clickedDays: currDays }) }}> {name} </a>
                        {idx !== datesLength - 1 && <> | </>}
                    </span>
                )}
                </p>
            </header>
            {status === "LOADING" &&
                <div className="full-screen-center">
                    <Loader />
                </div>}
            {status === "ERROR" &&
                <div className="full-screen-center">
                    <div className="alert alert-danger" role="alert">
                        <p>Something went wrong! 😢</p>
                        {errorMsg != null && <p>{errorMsg}</p>}
                    </div>
                </div>}
            {status === "DONE" && dashboardData != null && <>
                <div className="row">
                    <div className="col col-lg-12 col-md-12 col-sm-12">
                        <div className="card">
                            <div className="card-body">
                                <ReactApexChart type="area" options={DNS_REQUEST_OPTIONS} series={dnsRequestsData} height={350} />
                            </div>
                        </div>
                    </div>
                </div>
                <div className="row">
                    <div className="col col-lg-12 col-md-12 col-sm-12">
                        <div className="card">
                            <div className="card-body">
                                <ReactApexChart type="bar" options={MS_OPTIONS} series={msData} height={350} />
                            </div>
                        </div>
                    </div>
                </div>
                <div className="row">
                    <div className="col col-lg-4 col-md-6 col-sm-12">
                        <div className="card">
                            <div className="card-header">
                                Top Approved Websites
                            </div>
                            <div className="card-body">
                                <table className="table table-striped table-sm">
                                    <thead>
                                        <tr>
                                            <th></th>
                                            <th>Name</th>
                                            <th>Count</th>
                                        </tr>
                                    </thead>
                                    <tbody>
                                        {Object.entries(dashboardData?.top_approved).map(([name, count], idx) => <tr key={idx}>
                                            <td>{idx + 1}</td>
                                            <td>{name}</td>
                                            <td>{count}</td>
                                        </tr>)}
                                    </tbody>
                                </table>
                            </div>
                        </div>
                    </div>
                    <div className="col col-lg-4 col-md-6 col-sm-12">
                        <div className="card">
                            <div className="card-header">
                                Request Types
                            </div>
                            <div className="card-body">
                                <ReactApexChart
                                    options={{ ...PIE_OPTIONS, labels: Object.keys(dashboardData.queries ?? {}) }}
                                    series={Object.values(dashboardData?.queries ?? {})}
                                    type="pie"
                                    height={370} />
                            </div>
                        </div>
                    </div>
                    <div className="col col-lg-4 col-md-6 col-sm-12">
                        <div className="card">
                            <div className="card-header">
                                Top Blocked Websites
                            </div>
                            <div className="card-body">
                                <table className="table table-striped table-sm">
                                    <thead>
                                        <tr>
                                            <th></th>
                                            <th>Name</th>
                                            <th>Count</th>
                                        </tr>
                                    </thead>
                                    <tbody>
                                        {Object.entries(dashboardData?.top_rejected).map(([name, count], idx) => <tr key={idx}>
                                            <td>{idx + 1}</td>
                                            <td>{name}</td>
                                            <td>{count}</td>
                                        </tr>)}
                                    </tbody>
                                </table>
                            </div>
                        </div>
                    </div>
                </div>
            </>}
        </section >
    );
}

const DNS_REQUEST_OPTIONS = {
    chart: {
        animations: { enabled: false },
        height: 350,
        stacked: true,
        type: 'area',
        zoom: { enabled: true }
    },
    colors: ['#ff0000', '#fb6e87', '#22c522', '#8d4bf3'],
    dataLabels: { enabled: false },
    title: { text: 'DNS Requests', align: 'left' },
    tooltip: {
        x: {
            format: 'HH:mm MMM, dd'
        }
    },
    stroke: { curve: 'smooth', width: 2 },
    xaxis: { type: 'datetime' }
};

const MS_OPTIONS = {
    chart: {
        animations: { enabled: false },
        height: 350,
        zoom: { enabled: true }
    },
    colors: ['#fb6e87', '#22c522', '#8d4bf3'],
    dataLabels: { enabled: false },
    title: { text: 'Response Time (ms)', align: 'left' },
    tooltip: {
        x: {
            format: 'HH:mm MMM, dd'
        }
    },
    stroke: { curve: 'straight', width: 3 },
    xaxis: { type: 'datetime' }
};

const PIE_OPTIONS = {
    legend: {
        position: 'bottom'
    }
};
