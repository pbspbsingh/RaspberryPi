import { Action } from "./State";

let connectionInit = false;
let socket: WebSocket | null = null;
let dispatch: React.Dispatch<Action> | null = null;

export default function connectWs(d: React.Dispatch<Action>) {
    dispatch = d;
    if (connectionInit) {
        // console.debug("WS connection already initialized");
        return;
    }
    connectionInit = true;
    connect();
}

function connect() {
    if (socket != null) {
        socket.close();
    }

    let { hostname, port } = window.location;
    if (port === "3000") {
        port = "8080";
    }
    if (port === "") {
        socket = new WebSocket(`ws://${hostname}/websocket`);
    } else {
        socket = new WebSocket(`ws://${hostname}:${port}/websocket`);
    }
    socket.onopen = () => console.log("Websocket connected successfully!");
    socket.onclose = () => {
        console.log("Websocket disconnected, lets try after 5 seconds");
        socket = null;
        setTimeout(connect, 5000);
    };
    socket.onerror = (e) => {
        console.warn("Something went wrong with websocket", e);
        socket?.close();
        socket = null;
    };
    socket.onmessage = ({ data }) => {
        console.debug("Got message on websocket", data);
        if (data != null && dispatch != null) {
            try {
                const payload = JSON.parse(data);
                if (payload.query != null) {
                    // console.debug("Got a new query object, dispatching it");
                    dispatch({ type: "NEW_QUERY", newQuery: payload.query });
                }
            } catch (e) {
                console.warn("Failed to parse ws message", e);
            }
        }
    }
}