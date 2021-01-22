import React, { useState } from 'react';

const DATE_RANGE = {
    "1 Day": 1,
    "3 Days": 3,
    "7 Days": 7,
    "14 Days": 14,
};

export default function Dashboard(): JSX.Element {
    const [selectedDate, updateDate] = useState("1 Day");
    const dateRangeLength = Object.keys(DATE_RANGE).length;
    return (
        <section>
            <header>
                <h2>Dashboard</h2>
                <p className="filter-date-range">
                    Date Range:
                    {Object.entries(DATE_RANGE).map(([name, days], idx) =>
                    <span key={idx}>
                        <a href="#" className={selectedDate === name ? "selected" : ""} onClick={() => onDateChange(name)}> {name} </a>
                        {idx !== dateRangeLength - 1 && <> | </>}
                    </span>
                )}
                </p>
            </header>
            <div className="row">

            </div>
        </section>
    );
}

function onDateChange(newDate: string) {

}