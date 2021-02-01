import React, { useEffect, useReducer } from 'react';
import { Toast as Toaster } from 'bootstrap';
import { Loader } from '../Icons';

type ConfigState = {
    status: "LOADING" | "ERROR" | "DONE",
    approveRules: string[],
    rejectRules: string[],
    blockList: Array<[string, number]>
    updateBtnEnabled: boolean,
    updated?: boolean,
};

type Action = {
    type: "LOADED",
    approveRules: string[],
    rejectRules: string[],
    blockList: Array<[string, number]>
} | {
    type: "ERROR",
} | {
    type: "PROGRESS",
} | {
    type: "UPDATED",
    updated: boolean,
};

function reduce(state: ConfigState, action: Action): ConfigState {
    switch (action.type) {
        case "LOADED": {
            return { ...state, ...action, status: "DONE", updateBtnEnabled: true };
        }
        case "ERROR": {
            return { ...state, status: "ERROR" };
        }
        case "PROGRESS": {
            return { ...state, updateBtnEnabled: false };
        }
        case "UPDATED": {
            return { ...state, updated: action.updated, updateBtnEnabled: true };
        }
        default: throw new Error("Unsupported reduction: " + action);
    }
}

let updateToast: Toaster | null = null;

export default function Config(): JSX.Element {
    const [state, dispatch] = useReducer(reduce, {
        status: "LOADING",
        approveRules: [],
        rejectRules: [],
        blockList: [],
        updateBtnEnabled: false,
    });
    const { status, approveRules, rejectRules, blockList, updated } = state;
    useEffect(() => {
        loadConfig(dispatch);
        updateToast = new Toaster(document.getElementById("updateToast")!!, {});
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
            <Toast updated={updated} />
            {status === "DONE" &&
                <form action="/config" method="post" onSubmit={e => updateConfig(e, dispatch)}>
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
                    <div className="row">
                        <div className="col col-lg-6 col-md-6 col-sm-12">
                            <div className="card">
                                <div className="card-header">
                                    Current Block List
                                </div>
                                <div className="card-body">
                                    <table className="table table-striped table-sm">
                                        <thead>
                                            <tr>
                                                <th>Source</th>
                                                <th>Count</th>
                                            </tr>
                                        </thead>
                                        <tbody>
                                            {blockList.map(([source, count], idx) => <tr key={idx}>
                                                <td>{source}</td>
                                                <td>{count}</td>
                                            </tr>)}
                                        </tbody>
                                    </table>
                                </div>
                            </div>
                        </div>
                        <div className="col col-lg-6 col-md-6 col-sm-12">
                            <div className="card">
                                <div className="card-header">
                                    Update Block List
                                </div>
                                <div className="card-body">
                                    <textarea
                                        className="form-control"
                                        style={{ marginTop: "25px", lineHeight: 2.07 }}
                                        name="updatedBlockList"
                                        defaultValue={blockList.map(([source]) => source).join("\n")}
                                        rows={blockList.length + 2} />
                                </div>
                            </div>
                        </div>
                        <div className="row">
                            <div className="col d-flex flex-column justify-content-center">
                                <input type="submit" value="Update" disabled={!state.updateBtnEnabled} />
                            </div>
                        </div>
                    </div>
                </form>
            }
        </section>
    );
}

const Toast = ({ updated }: { updated?: boolean }) => (
    <div className="toast-container position-fixed" style={{ zIndex: 10, right: "50px" }}>
        <div id="updateToast" className="toast" role="alert" aria-live="assertive" aria-atomic="true">
            <div className="toast-header">
                <strong className="me-auto">Config status</strong>
                <button type="button" className="btn-close ms-auto me-2" data-bs-dismiss="toast" aria-label="Close"></button>
            </div>
            <div className="toast-body">
                {false === updated && <div className="alert alert-danger" role="alert">
                    <p>Failed to updated! 😥</p>
                </div>}
                {true === updated && <div className="alert alert-success center-align" role="alert">
                    <p>Updated Successfully! 😎</p>
                </div>}
            </div>
        </div>
    </div>
);

async function loadConfig(dispatch: React.Dispatch<Action>) {
    try {
        const request = await fetch('/config');
        const response = await request.json();
        dispatch({
            type: "LOADED",
            approveRules: response.approve_rules,
            rejectRules: response.reject_rules,
            blockList: response.block_list,
        });
    } catch (e) {
        console.warn("Fetching config failed", e);
        dispatch({ type: "ERROR" });
    }
}

async function updateConfig(e: React.FormEvent, dispatch: React.Dispatch<Action>) {
    e.preventDefault();

    updateToast?.hide();
    dispatch({ type: "PROGRESS" });

    const formData = new FormData(e.target as HTMLFormElement)
    try {
        const request = await fetch('/config', {
            method: 'post',
            body: new URLSearchParams([...formData as any])
        });
        if (request.status === 200) {
            dispatch({ type: "UPDATED", updated: true });
        } else {
            throw new Error("Config update failed: " + request.status);
        }
    } catch (e) {
        console.error("failed to update config", e);
        dispatch({ type: "UPDATED", updated: false });
    }
    updateToast?.show();
}
