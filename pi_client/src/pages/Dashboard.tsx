import React, { useContext, useEffect } from 'react';
import ReactApexChart from 'react-apexcharts';

import { Loader } from '../Icons';
import { AppContext, DATE_RANGE } from '../State';

import load from '../requests';

export default function Dashboard(): JSX.Element {
    const { state, dispatch } = useContext(AppContext);
    const { status, errorMsg, dashboardData } = state;

    useEffect(() => {
        if (dashboardData == null) {
            loadDashboard(dispatch, state.days);
        }
    }, []);

    const dnsRequestsData = [{
        name: "Rejected",
        data: dashboardData?.rejected
    }, {
        name: "Approved",
        data: dashboardData?.approved
    }, {
        name: "Passed",
        data: dashboardData?.passed
    }];
    const msData = [{
        name: "Rejected",
        data: dashboardData?.rejected_ms
    }, {
        name: "Approved",
        data: dashboardData?.approved_ms
    }, {
        name: "Passed",
        data: dashboardData?.passed_ms
    }];
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
                    {Object.entries(DATE_RANGE).map(([days, name], idx) =>
                    <span key={idx}>
                        <a href="#" className={state.days === days ? "selected" : ""}
                            onClick={() => loadDashboard(dispatch, days)}> {name} </a>
                        {idx !== datesLength - 1 && <> | </>}
                    </span>
                )}
                </p>
            </header>
            {status === "LOADING" && <div className="full-screen-center"><Loader /></div>}
            {status === "ERROR" && <div className="full-screen-center">
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
                                    options={{ labels: Object.keys(dashboardData.queries ?? {}) }}
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

async function loadDashboard(dispatcher: React.Dispatch<any>, days: string) {
    dispatcher({ type: "SET_LOADING" });
    try {
        const request = await load(`/dashboard/${days}`);
        if (request == null) return;
        dispatcher({ type: "UPDATE_DASHBOARD", days, dashboardData: await request.json() });
    } catch (e) {
        console.warn(e);
        dispatcher({ type: "SET_ERROR", errorMsg: e.message });
    }
}

const DNS_REQUEST_OPTIONS = {
    chart: {
        height: 350,
        stacked: true,
        type: 'area',
        zoom: { enabled: true }
    },
    colors: ['#ff49d7', '#66da26', '#6389e0'],
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
        height: 350,
        zoom: { enabled: true }
    },
    colors: ['#ff49d7', '#66da26', '#6389e0'],
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


