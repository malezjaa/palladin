import {StrictMode} from "react";
import {createRoot} from "react-dom/client";
import {App} from "./App";

const elem = document.getElementById("root")!;
const app = (
    <StrictMode>
        <App/>
    </StrictMode>
);

createRoot(elem).render(app);
