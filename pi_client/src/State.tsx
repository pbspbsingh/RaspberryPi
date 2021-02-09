import React, { createContext, useReducer } from "react";

export const DATE_RANGE = {
    "1": "1 Day",
    "3": "3 Days",
    "7": "7 Days",
    "14": "14 Days",
}

export const QUERY_SIZE = [100, 200, 300, 400, 500];

export type AppAction = {
    type: "SET_LOADING"
} | {
    type: "CLICKED_DAYS",
    clickedDays: string,
} | {
    type: "SET_ERROR",
    errorMsg: string,
} | {
    type: "UPDATE_DASHBOARD",
    days: string,
    dashboardData: DashboardData,
} | {
    type: "UPDATE_QUERIES",
    querySize: number,
    queries: DnsQuery[],
} | {
    type: "NEW_QUERY",
    newQuery: DnsQuery,
} | {
    type: "UPDATE_HEALTH",
    days: string,
    health: Array<{ name: string, data: Array<[number, number]> }>
} | {
    type: "NEW_HEALTH",
    newHealth: {
        time: number,
        cpu_avg?: number,
        cpu_temp?: number,
        memory?: number,
    }
};

export interface AppState {
    clickedDays: string,
    days: string,
    status: "LOADING" | "DONE" | "ERROR",
    errorMsg?: String,
    dashLastUpdated: number,
    dashboardData?: DashboardData,
    querySize: number,
    queries?: DnsQuery[],
    health?: Array<{ name: string, data: Array<[number, number]> }>
}

export interface DashboardData {
    total_count: number,
    reject_count: number,
    dns_data: Array<{ name: string, data: Array<[number, number]> }>,
    latency_data: Array<{ name: string, data: Array<[number, number]> }>
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
    clickedDays: "1",
    days: "1",
    status: "LOADING",
    dashLastUpdated: 0,
    querySize: 100,
}

export function appReducer(state: AppState, action: AppAction): AppState {
    switch (action.type) {
        case "SET_LOADING": {
            return { ...state, status: "LOADING" };
        }
        case "CLICKED_DAYS": {
            return { ...state, clickedDays: action.clickedDays };
        }
        case "SET_ERROR": {
            return { ...state, status: "ERROR", errorMsg: action.errorMsg };
        }
        case "UPDATE_DASHBOARD": {
            return {
                ...state,
                status: "DONE",
                errorMsg: undefined,
                days: action.days,
                dashLastUpdated: Date.now(),
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

            let queries = [action.newQuery, ...state.queries.slice(0, -1)];
            if (queries.length > state.querySize) {
                console.warn(`Queries size found ${queries.length} when expected was ${state.querySize}`);
                queries = queries.slice(0, state.querySize);
            }
            return { ...state, queries }
        }
        case "UPDATE_HEALTH": {
            return {
                ...state,
                status: "DONE",
                errorMsg: undefined,
                days: action.days,
                health: action.health,
            };
        }
        case "NEW_HEALTH": {
            const health = state.health;
            if (health == null || health.length != 3) { return state; }
            const { time, cpu_avg, memory, cpu_temp } = action.newHealth;
            if (cpu_avg != null) {
                health[0].data.push([time, cpu_avg]);
            }
            if (memory != null) {
                health[1].data.push([time, memory]);
            }
            if (cpu_temp != null) {
                health[2].data.push([time, cpu_temp]);
            }
            return { ...state, health };
        }
        default: {
            console.log('Unexpected action:', action);
        }
    }
    return state;
}

export const AppContext = createContext({
    state: INITIAL_STATE,
    dispatch: (_action: AppAction) => { }
});

export function AppContextProvider(props: React.Props<any>) {
    const [state, dispatch] = useReducer(appReducer, INITIAL_STATE);
    return (
        <AppContext.Provider value={{ state, dispatch }}>
            {props.children}
        </AppContext.Provider>
    );
}