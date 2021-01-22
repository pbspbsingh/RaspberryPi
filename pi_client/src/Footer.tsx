import React, { useEffect, useRef } from 'react';
// import { createPopperLite } from '@popperjs/core';

export default function Footer(): JSX.Element {
    const about = useRef<HTMLAnchorElement>(null);

    return (
        <footer className="d-flex flex-row justify-content-between">
            <p><a href="https://github.com/pbspbsingh/RaspberryPi">Github</a></p>
            <p>Raspberry Pi Utils</p>
            <p>
                <a href="#" ref={about} data-animation="true" data-container="body"
                    data-toggle="popover" data-title="About" data-content="Welcome to Raspberry Utils">
                    About
                </a>
            </p>
        </footer>
    );
}