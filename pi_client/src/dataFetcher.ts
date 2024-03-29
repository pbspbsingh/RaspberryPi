import { useContext, useEffect } from "react";
import { AppAction, AppContext, DnsQuery } from "./State";

let wsInitialized = false;

export function useDataFetcher() {
    const { dispatch } = useContext(AppContext);

    function connectWs() {
        let { hostname, port } = window.location;
        if (port === "3000") {
            port = "8080";
        }

        let socket: WebSocket;
        if (port === "") {
            socket = new WebSocket(`ws://${hostname}/websocket`);
        } else {
            socket = new WebSocket(`ws://${hostname}:${port}/websocket`);
        }
        socket.onopen = () => console.log("Websocket connected successfully!");
        socket.onclose = () => {
            console.log("Websocket disconnected, lets try after 10 seconds");
            setTimeout(connectWs, 10000);
        };
        socket.onerror = (e) => {
            console.warn("Something went wrong with websocket", e);
            socket.close();
        };
        socket.onmessage = ({ data }) => {
            // console.debug("Got message on websocket", data);
            if (data != null) {
                try {
                    const payload = JSON.parse(data);
                    if (payload.query != null) {
                        dispatch({ type: "NEW_QUERY", newQuery: payload.query });
                    }
                    if (payload.health != null) {
                        dispatch({ type: "NEW_HEALTH", newHealth: payload.health });
                    }
                } catch (e) {
                    console.warn("Failed to parse ws message", e);
                }
            }
        }
    }

    useEffect(() => {
        if (wsInitialized) {
            console.error("Websocket is initialized already, but useEffect got called anyways");
        }
        else {
            wsInitialized = true;
            connectWs();
        }
    }, []);
}

export async function loadDashboard(dispatch: React.Dispatch<AppAction>, days: string) {
    try {
        const request = await fetch(`/dashboard/${days}`);
        dispatch({ type: "UPDATE_DASHBOARD", days, dashboardData: await request.json() });
    } catch (e: any) {
        console.warn(e);
        dispatch({ type: "SET_ERROR", errorMsg: e.message });
    }
}

export async function loadQuery(dispatch: React.Dispatch<AppAction>, querySize: number) {
    dispatch({ type: "SET_LOADING" });
    try {
        const request = await fetch(`/queries/${querySize}`);
        const queries: DnsQuery[] = await request.json();
        dispatch({ type: "UPDATE_QUERIES", querySize, queries });
    } catch (e: any) {
        console.warn(e);
        dispatch({ type: "SET_ERROR", errorMsg: e.message });
    }
}

export async function loadHealth(dispatch: React.Dispatch<AppAction>, days: string) {
    dispatch({ type: "SET_LOADING" });
    try {
        const request = await fetch(`/health/${days}`);
        const health: Array<{ name: string, data: [number, number][] }> = await request.json();
        dispatch({ type: "UPDATE_HEALTH", days, health });
    } catch (e: any) {
        console.warn(e);
        dispatch({ type: "SET_ERROR", errorMsg: e.message });
    }
}