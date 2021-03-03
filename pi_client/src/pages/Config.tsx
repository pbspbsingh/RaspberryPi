import React, { useEffect, useReducer } from 'react';
import { Toast as Toaster } from 'bootstrap';
import { Loader, Trash, Undo } from '../Icons';

type ConfigState = {
    status: "LOADING" | "ERROR" | "DONE",
    approveRules: string[],
    rejectRules: string[],
    blockList: Array<[string, number, boolean]>
    updateBtnEnabled: boolean,
    updated?: boolean,
};

type Action = {
    type: "LOADED",
    approveRules: string[],
    rejectRules: string[],
    blockList: Array<[string, number, boolean]>
} | {
    type: "TOGGLE_DELETED",
    deleteIdx: number,
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
        case "TOGGLE_DELETED": {
            const blockList = state.blockList.slice(0, action.deleteIdx);
            const [url, count, keep] = state.blockList[action.deleteIdx];
            blockList.push([url, count, !keep]);
            blockList.push(...state.blockList.slice(action.deleteIdx + 1));
            return { ...state, blockList };
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
                    <div className="row">
                        <div className="col">
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
                                                <th className="text-center">Action</th>
                                            </tr>
                                        </thead>
                                        <tbody>
                                            {blockList.map(([source, count, keep], idx) => <tr key={idx} className={!keep ? "strikeout" : ""}>
                                                <td>{source}</td>
                                                <td>{count}</td>
                                                <td className="text-center" style={{ cursor: "pointer" }}
                                                    onClick={() => dispatch({ type: "TOGGLE_DELETED", deleteIdx: idx })}>
                                                    {keep ? <Trash /> : <Undo />}
                                                </td>
                                            </tr>)}
                                        </tbody>
                                    </table>
                                </div>
                            </div>
                        </div>
                    </div>
                    <div className="row">
                        <div className="col">
                            <div className="card">
                                <div className="card-header">
                                    New Entries for Block List
                                    </div>
                                <div className="card-body">
                                    <textarea
                                        className="form-control"
                                        name="newBlockListEntries"
                                        rows={5} />
                                </div>
                            </div>
                        </div>
                    </div>
                    <div className="row">
                        <div className="col d-flex flex-column justify-content-center">
                            <input type="submit" value="Update" disabled={!state.updateBtnEnabled} />
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
            blockList: response.block_list.map(([url, count]: [string, number]) => [url, count, true]),
        });
    } catch (e) {
        console.warn("Fetching config failed", e);
        dispatch({ type: "ERROR" });
    }
}

async function updateConfig(e: React.FormEvent, dispatch: React.Dispatch<Action>, state: ConfigState) {
    e.preventDefault();

    updateToast?.hide();
    dispatch({ type: "PROGRESS" });

    const param = new URLSearchParams();
    const approveRules = (document.forms[0].querySelector("textarea[name=approveRules]") as HTMLTextAreaElement).value;
    const rejectRules = (document.forms[0].querySelector("textarea[name=rejectRules]") as HTMLTextAreaElement).value;
    const newBlockListEntries = (document.forms[0].querySelector("textarea[name=newBlockListEntries]") as HTMLTextAreaElement).value;
    const keptBLockList = state.blockList.filter(item => item[2]).map(item => item[0]).join("\n");

    param.append("approveRules", approveRules);
    param.append("rejectRules", rejectRules);
    param.append("updatedBlockList", [keptBLockList, newBlockListEntries].join("\n"));
    try {
        const request = await fetch('/config', { method: 'post', body: param });
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
