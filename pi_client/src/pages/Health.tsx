import React, { useContext, useEffect } from 'react';
import ReactApexChart from 'react-apexcharts';

import { Loader } from '../Icons';
import { AppContext, DATE_RANGE } from '../State';
import { loadHealth } from '../dataFetcher';

export default function Health(): JSX.Element {
    const { state, dispatch } = useContext(AppContext);
    const { status, clickedDays, errorMsg, health, days } = state;

    useEffect(() => {
        if (health == null || clickedDays !== days) {
            loadHealth(dispatch, clickedDays);
        }
    }, [clickedDays]);

    const datesLength = Object.keys(DATE_RANGE).length;
    return (
        <section className="h-100">
            <header>
                <div className="d-flex align-items-center">
                    <h2>Health</h2> &nbsp;
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
            {status === "DONE" && health != null && <>
                {health.map((series, idx) => <div className="row" key={series.name}>
                    <div className="col col-lg-12 col-md-12 col-sm-12">
                        <div className="card">
                            <div className="card-body">
                                <ReactApexChart
                                    type="area" height={300}
                                    options={chartOptions(series.name, idx)}
                                    series={[series]} />
                            </div>
                        </div>
                    </div>
                </div>)}
            </>}
        </section>
    );
}

const COLORS: string[] = ['#ff49d7', '#66da26', '#6389e0'];

const chartOptions = (name: string, idx: number) => ({
    chart: {
        animations: { enabled: false },
        height: 350,
        zoom: { enabled: true }
    },
    colors: [COLORS[idx]],
    dataLabels: { enabled: false },
    title: { text: name, align: 'left' },
    tooltip: {
        x: {
            format: 'HH:mm MMM, dd'
        }
    },
    stroke: { curve: idx === 1 ? 'smooth' : 'straight', width: 3 },
    xaxis: { type: 'datetime' }
});