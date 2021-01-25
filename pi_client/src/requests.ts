let lastRequest = -1;

export default async function load(url: string): Promise<Response | null> {
    lastRequest = Date.now();
    const requestTime = lastRequest;
    try {
        const response = await fetch(url);
        if (requestTime !== lastRequest) {
            console.warn("A new request has been made after this one, cancelling this one.")
            return null;
        } else {
            return response;
        }
    } catch (e) {
        if (requestTime !== lastRequest) {
            console.warn("Previous request was aborted, returning null")
            return null;
        } else {
            throw e;
        }
    }
}