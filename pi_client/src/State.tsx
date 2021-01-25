import React, { createContext, useReducer } from "react";

export const DATE_RANGE = {
    "1": "1 Day",
    "3": "3 Days",
    "7": "7 Days",
    "14": "14 Days",
}

export type Action = {
    type: "SET_LOADING" | "SET_ERROR" | "UPDATE_DASHBOARD",
} & Partial<AppState>;

export interface AppState {
    days: keyof typeof DATE_RANGE,
    status: "LOADING" | "DONE" | "ERROR",
    errorMsg?: String,
    dashboardData?: DashboardData
}

export interface DashboardData {
    total_count: number,
    reject_count: number,
    passed: [[number, number]],
    approved: [[number, number]],
    rejected: [[number, number]],
    passed_ms: [[number, number]],
    approved_ms: [[number, number]],
    rejected_ms: [[number, number]],
    queries: { [key: string]: number },
    top_approved: { [key: string]: number },
    top_rejected: { [key: string]: number },
}

export const INITIAL_STATE: AppState = {
    days: "1",
    status: "LOADING",
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
        default: {
            console.log('Unexpected action:', action);
        }
    }
    return state;
}

export const AppContext = createContext({
    state: INITIAL_STATE,
    dispatch: (action: Action) => { }
});

export function AppContextProvider(props: React.Props<any>) {
    const [state, dispatch] = useReducer(appReducer, INITIAL_STATE);
    return (
        <AppContext.Provider value={{ state, dispatch }}>
            {props.children}
        </AppContext.Provider>
    );
}