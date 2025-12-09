import React, { useState, FormEvent } from 'react';

interface ChatInputProps {
    onSendMessage: (message: string) => void;
    isLoading: boolean;
}

export default function ChatInput({ onSendMessage, isLoading }: ChatInputProps) {
    const [message, setMessage] = useState<string>('');

    const handleSubmit = (e: FormEvent) => {
        e.preventDefault();

        if (!message.trim()) return;

        onSendMessage(message);
        setMessage('');
    };

    return (
        <form
            onSubmit={handleSubmit}
            className="bg-white border-t border-gray-200 p-4 sticky bottom-0"
        >
            <div className="flex items-center">
                <input
                    type="text"
                    value={message}
                    onChange={(e) => setMessage(e.target.value)}
                    placeholder="Type your message here..."
                    disabled={isLoading}
                    className="flex-1 p-3 border border-gray-300 rounded-l-md focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                />
                <button
                    type="submit"
                    disabled={isLoading || !message.trim()}
                    className={`px-4 py-3 rounded-r-md text-white font-medium focus:outline-none ${isLoading || !message.trim()
                            ? 'bg-gray-400 cursor-not-allowed'
                            : 'bg-blue-600 hover:bg-blue-700'
                        }`}
                >
                    {isLoading ? (
                        <span className="flex items-center">
                            <svg className="animate-spin -ml-1 mr-2 h-4 w-4 text-white" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
                                <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4"></circle>
                                <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z"></path>
                            </svg>
                            Processing...
                        </span>
                    ) : (
                        'Send'
                    )}
                </button>
            </div>
        </form>
    );
}