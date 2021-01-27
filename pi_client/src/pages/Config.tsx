import React, { useContext, useEffect, useState } from 'react';
import { Loader } from '../Icons';
import { AppContext } from '../State';

type ConfigState = {
    status: "LOADING" | "ERROR" | "DONE",
    approveRules: string[],
    rejectRules: string[],
    updated?: boolean,
    updateBtnEnabled: boolean,
};

const initialState: ConfigState = {
    status: "LOADING",
    approveRules: [],
    rejectRules: [],
    updateBtnEnabled: true,
}

export default function Config(): JSX.Element {
    const [state, dispatch] = useState(initialState)
    const { status, approveRules, rejectRules, updated } = state;
    useEffect(() => {
        loadConfig(dispatch);
    }, []);

    return (
        <section className="h-100">
            <header>
                <h2>Block Configuration</h2>
                <p className="filter-date-range">
                    Prefix with * for regex, # to disable a rule
                </p>
            </header>
            {status === "LOADING" && <div className="full-screen-center">
                <Loader />
            </div>}
            {status === "ERROR" && <div className="full-screen-center">
                <div className="alert alert-danger" role="alert">
                    <p>Something went wrong! 😢</p>
                </div>
            </div>}
            {status === "DONE" &&
                <form action="/config" method="post" onSubmit={e => updateConfig(e, dispatch, state)}>
                    <div className="row">
                        <div className="col col-lg-6 col-md-6 col-sm-12">
                            <div className="card">
                                <div className="card-header">
                                    Approve Rules
                            </div>
                                <div className="card-body">
                                    <textarea
                                        className="form-control"
                                        name="approveRules"
                                        defaultValue={approveRules.join("\n")}
                                        rows={Math.max(approveRules.length, rejectRules.length)} />
                                </div>
                            </div>
                        </div>
                        <div className="col col-lg-6 col-md-6 col-sm-12">
                            <div className="card">
                                <div className="card-header">
                                    Reject Rules
                            </div>
                                <div className="card-body">
                                    <textarea
                                        className="form-control"
                                        name="rejectRules"
                                        defaultValue={rejectRules.join("\n")}
                                        rows={Math.max(approveRules.length, rejectRules.length)} />
                                </div>
                            </div>
                        </div>
                    </div>
                    <div className="row">
                        <div className="col d-flex flex-column justify-content-center">
                            <input type="submit" value="Update" disabled={!state.updateBtnEnabled} />
                        </div>
                    </div>
                    {updated != null && <div className="row">
                        <div className="col d-flex justify-content-center">
                            {false === updated && <div className="alert alert-danger" role="alert">
                                <p>Failed to updated! 😥</p>
                            </div>}
                            {true === updated && <div className="alert alert-success" role="alert">
                                <p>Updated Successfully! 😎</p>
                            </div>}
                        </div>
                    </div>}
                </form>
            }
        </section>
    );
}

async function loadConfig(distpach: React.Dispatch<ConfigState>) {
    try {
        const request = await fetch('/config');
        const response = await request.json();
        distpach({
            ...initialState,
            status: "DONE",
            approveRules: response.approve_rules,
            rejectRules: response.reject_rules,
        });
    } catch (e) {
        console.warn("Fetching config failed", e);
        distpach({ ...initialState, status: "ERROR" });
    }
}

async function updateConfig(e: React.FormEvent, dispatch: React.Dispatch<ConfigState>, state: ConfigState) {
    e.preventDefault();

    dispatch({ ...state, updateBtnEnabled: false });
    const formData = new FormData(e.target as HTMLFormElement)
    try {
        const response = await fetch('/config', {
            method: 'post',
            body: new URLSearchParams([...formData as any])
        });
        if (response.status === 200) {
            dispatch({ ...state, updated: true, updateBtnEnabled: true });
        } else {
            dispatch({ ...state, updated: false, updateBtnEnabled: true });
        }
    } catch (e) {
        console.error("failed to update config", e);
        dispatch({ ...state, updated: false, updateBtnEnabled: true });
    }
}
