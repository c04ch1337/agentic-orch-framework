'use client';

import React from 'react';
import ChatContainer from '@/components/dashboard/ChatContainer';

export default function Dashboard() {
    return (
        <div className="min-h-[calc(100vh-64px)]">
            <div className="container mx-auto">
                <div className="py-4 px-4 sm:px-6">
                    <h1 className="text-2xl font-bold text-gray-800 mb-1">Dashboard</h1>
                    <p className="text-gray-600 mb-4">
                        Interact with the Phoenix ORCH system using the chat interface below.
                    </p>

                    <div className="bg-white rounded-lg shadow-md border border-gray-200 overflow-hidden">
                        <ChatContainer />
                    </div>
                </div>
            </div>
        </div>
    );
}