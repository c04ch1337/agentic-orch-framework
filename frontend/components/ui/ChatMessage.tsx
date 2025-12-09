import React from 'react';
import { Message } from '@/types';

interface ChatMessageProps {
    message: Message;
    expanded: boolean;
    onToggleExpand: () => void;
}

export default function ChatMessage({ message, expanded, onToggleExpand }: ChatMessageProps) {
    // Helper to format the date
    const formatDate = (date: Date) => {
        return new Intl.DateTimeFormat('en-US', {
            hour: '2-digit',
            minute: '2-digit',
            second: '2-digit',
        }).format(date);
    };

    return (
        <div className={`p-4 mb-4 rounded-lg ${message.isUser ? 'bg-blue-50 ml-12' : 'bg-gray-50 mr-12'}`}>
            <div className="flex justify-between items-start mb-2">
                <div className="font-medium">
                    {message.isUser ? 'You' : 'Phoenix ORCH'}
                </div>
                <div className="text-xs text-gray-500">
                    {formatDate(message.timestamp)}
                </div>
            </div>

            <div className="mb-3 whitespace-pre-wrap">
                {message.content}
            </div>

            {!message.isUser && message.response && (
                <div className="mt-4 text-sm">
                    <div className="border-t pt-2 border-gray-200">
                        <div className="flex justify-between items-center mb-2">
                            <div className="font-medium text-gray-700">Execution Details</div>
                            <button
                                className="text-blue-600 text-xs hover:underline focus:outline-none"
                                onClick={onToggleExpand}
                            >
                                {expanded ? 'Hide Details' : 'Show Details'}
                            </button>
                        </div>

                        {expanded && (
                            <div className="space-y-2 px-3 py-2 bg-gray-100 rounded-md">
                                <div>
                                    <span className="text-gray-600 font-medium">Routed To:</span>{' '}
                                    <span className="text-gray-800">{message.response.metadata.routedTo}</span>
                                </div>

                                <div>
                                    <span className="text-gray-600 font-medium">Plan:</span>
                                    <ul className="list-disc pl-5 mt-1">
                                        {message.response.metadata.plan.map((step, index) => (
                                            <li key={index} className="text-gray-800">{step}</li>
                                        ))}
                                    </ul>
                                </div>

                                <div>
                                    <span className="text-gray-600 font-medium">Thought Process:</span>
                                    <div className="mt-1 p-2 bg-white rounded border border-gray-200 max-h-48 overflow-auto">
                                        <pre className="text-xs whitespace-pre-wrap">{message.response.metadata.thoughtProcess}</pre>
                                    </div>
                                </div>
                            </div>
                        )}
                    </div>
                </div>
            )}
        </div>
    );
}