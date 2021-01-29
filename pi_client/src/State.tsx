import React, { createContext, useReducer } from "react";
import connectWs from "./websocket";

export const DATE_RANGE = {
    "1": "1 Day",
    "3": "3 Days",
    "7": "7 Days",
    "14": "14 Days",
}

export const QUERY_SIZE = [100, 200, 300, 400, 500];

export type Action = {
    type: "SET_LOADING" | "SET_ERROR" | "UPDATE_DASHBOARD" | "UPDATE_QUERIES" | "NEW_QUERY",
} & Partial<AppState>;

export interface AppState {
    days: keyof typeof DATE_RANGE,
    status: "LOADING" | "DONE" | "ERROR",
    errorMsg?: String,
    dashboardData?: DashboardData,
    querySize: number,
    queries?: DnsQuery[],
    newQuery?: DnsQuery,
}

export interface DashboardData {
    total_count: number,
    reject_count: number,
    passed: [number, number][],
    approved: [number, number][],
    rejected: [number, number][],
    passed_ms: [number, number][],
    approved_ms: [number, number][],
    rejected_ms: [number, number][],
    queries: { [key: string]: number },
    top_approved: { [key: string]: number },
    top_rejected: { [key: string]: number },
}

export interface DnsQuery {
    id: number,
    req_time: number,
    req_type: string,
    name: string,
    responded: boolean,
    reply?: string,
    filtered?: boolean,
    reason?: string,
    resp_time: number,
}

export const INITIAL_STATE: AppState = {
    days: "1",
    status: "LOADING",
    querySize: 100,
}

export function appReducer(state: AppState, action: Action): AppState {
    switch (action.type) {
        case "SET_LOADING": {
            return { ...state, status: "LOADING" };
        }
        case "SET_ERROR": {
            return { ...state, status: "ERROR", errorMsg: action.errorMsg };
        }
        case "UPDATE_DASHBOARD": {
            return {
                ...state,
                status: "DONE",
                errorMsg: undefined,
                days: action.days!!,
                dashboardData: action.dashboardData
            };
        }
        case "UPDATE_QUERIES": {
            return {
                ...state,
                errorMsg: undefined,
                status: "DONE",
                querySize: action.querySize!!,
                queries: action.queries
            };
        }
        case "NEW_QUERY": {
            if (state.queries == null || action.newQuery == null) {
                return state;
            }

            const queries = [action.newQuery, ...state.queries.slice(0, -1)];
            return { ...state, queries, newQuery: undefined }
        }
        default: {
            console.log('Unexpected action:', action);
        }
    }
    return state;
}

export const AppContext = createContext({
    state: INITIAL_STATE,
    dispatch: (_action: Action) => { }
});

export function AppContextProvider(props: React.Props<any>) {
    const [state, dispatch] = useReducer(appReducer, INITIAL_STATE);
    connectWs(dispatch);
    return (
        <AppContext.Provider value={{ state, dispatch }}>
            {props.children}
        </AppContext.Provider>
    );
}