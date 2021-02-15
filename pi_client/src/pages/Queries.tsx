import React, { useContext, useEffect } from 'react';
import { Loader } from '../Icons';
import { AppContext, QUERY_SIZE, DnsQuery } from '../State';
import { loadQuery } from '../dataFetcher';

export default function Queries(): JSX.Element {
    const { state, dispatch } = useContext(AppContext);
    const { status, querySize, queries, errorMsg } = state;
    useEffect(() => {
        if (queries == null) {
            loadQuery(dispatch, querySize);
        }
    }, []);

    return (
        <section className="h-100">
            <header>
                <h2>Queries</h2>
                <p className="filter-date-range">
                    Date Range:
                    {QUERY_SIZE.map((qsize, idx) =>
                    <span key={idx}>
                        <a href="#" className={querySize === qsize ? "selected" : ""}
                            onClick={() => loadQuery(dispatch, qsize)}> {qsize} </a>
                        {idx !== QUERY_SIZE.length - 1 && <> | </>}
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
            {status === "DONE" && queries != null && <>
                <div className="row">
                    <div className="col">
                        <table className="table table-sm queries">
                            <thead>
                                <tr>
                                    <th>Time</th>
                                    <th>Name</th>
                                    <th>Type</th>
                                    <th style={{width:"20%"}}>Response</th>
                                    <th>Reason</th>
                                    <th>Latency</th>
                                </tr>
                            </thead>
                            <tbody>
                                {tableContent(queries)}
                            </tbody>
                        </table>
                    </div>
                </div>
            </>}
        </section>
    );
}

function tableContent(queries: DnsQuery[]) {
    return queries.map(({ id, req_time, req_type, name, responded, filtered, reason, resp_time, reply }) => {
        const filterClass = filtered === true ? "approved" : filtered === false ? "blocked" : "";
        const respondedClass = responded === false ? "no-response" : "";
        return (<tr key={id} className={`${filterClass} ${respondedClass}`}>
            <td>{new Date(req_time).toISOString()}</td>
            <td>{name}</td>
            <td>{req_type}</td>
            <td className="text-truncate" style={{maxWidth: 0}} title={reply}>{reply}</td>
            <td>{reason}</td>
            <td className="text-right">{resp_time} ms</td>
        </tr>);
    });
}
