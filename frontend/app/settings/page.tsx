'use client';

import React, { useState, useEffect, FormEvent } from 'react';
import { testApiKey } from '@/lib/api/orchService';
import {
    saveApiKey,
    getApiKey,
    isValidApiKeyFormat
} from '@/lib/utils/apiKeyStorage';

export default function Settings() {
    const [apiKey, setApiKey] = useState<string>('');
    const [isValid, setIsValid] = useState<boolean | null>(null);
    const [isLoading, setIsLoading] = useState<boolean>(false);
    const [message, setMessage] = useState<{
        text: string;
        type: 'success' | 'error' | 'info' | null;
    }>({ text: '', type: null });

    // Load saved API key on mount
    useEffect(() => {
        const savedKey = getApiKey();
        if (savedKey) {
            setApiKey(savedKey);
            setIsValid(true); // Assume valid if it was previously saved
        }
    }, []);

    const validateApiKey = (key: string): boolean => {
        return isValidApiKeyFormat(key);
    };

    const handleInputChange = (e: React.ChangeEvent<HTMLInputElement>) => {
        const key = e.target.value;
        setApiKey(key);

        // Clear messages when typing
        if (message.text) {
            setMessage({ text: '', type: null });
        }

        // Clear validation state
        setIsValid(null);
    };

    const handleSubmit = async (e: FormEvent) => {
        e.preventDefault();

        setIsLoading(true);
        setMessage({ text: '', type: null });

        try {
            // Validate format
            const isFormatValid = validateApiKey(apiKey);
            if (!isFormatValid) {
                setIsValid(false);
                setMessage({
                    text: 'Invalid API key format. Please check and try again.',
                    type: 'error'
                });
                return;
            }

            // Test connection
            const connectionSuccessful = await testApiKey(apiKey);

            if (connectionSuccessful) {
                saveApiKey(apiKey);
                setIsValid(true);
                setMessage({
                    text: 'API key successfully validated and saved.',
                    type: 'success'
                });
            } else {
                setIsValid(false);
                setMessage({
                    text: 'Could not connect to the API with this key. Please check and try again.',
                    type: 'error'
                });
            }
        } catch (error) {
            setIsValid(false);
            setMessage({
                text: 'An error occurred while testing the API key.',
                type: 'error'
            });
        } finally {
            setIsLoading(false);
        }
    };

    return (
        <div className="min-h-[calc(100vh-64px)]">
            <div className="container mx-auto">
                <div className="py-4 px-4 sm:px-6">
                    <h1 className="text-2xl font-bold text-gray-800 mb-1">Settings</h1>
                    <p className="text-gray-600 mb-6">
                        Configure your Phoenix ORCH dashboard settings.
                    </p>

                    <div className="bg-white rounded-lg shadow-md border border-gray-200 p-6 max-w-2xl">
                        <h2 className="text-xl font-semibold mb-4">API Configuration</h2>

                        <form onSubmit={handleSubmit} className="space-y-4">
                            <div>
                                <label htmlFor="apiKey" className="block text-sm font-medium text-gray-700 mb-1">
                                    API Key
                                </label>
                                <input
                                    id="apiKey"
                                    type="password"
                                    value={apiKey}
                                    onChange={handleInputChange}
                                    placeholder="Enter your API key"
                                    className={`w-full p-3 border rounded-md focus:outline-none focus:ring-2 ${isValid === false
                                            ? 'border-red-300 focus:ring-red-500 focus:border-red-500'
                                            : isValid === true
                                                ? 'border-green-300 focus:ring-green-500 focus:border-green-500'
                                                : 'border-gray-300 focus:ring-blue-500 focus:border-blue-500'
                                        }`}
                                />
                                <p className="mt-1 text-sm text-gray-500">
                                    Your API key is stored securely in your browser.
                                </p>
                            </div>

                            {message.text && (
                                <div
                                    className={`p-3 rounded-md ${message.type === 'success' ? 'bg-green-50 text-green-800' :
                                            message.type === 'error' ? 'bg-red-50 text-red-800' :
                                                'bg-blue-50 text-blue-800'
                                        }`}
                                >
                                    {message.text}
                                </div>
                            )}

                            <div className="flex justify-end">
                                <button
                                    type="submit"
                                    disabled={isLoading || !apiKey.trim()}
                                    className={`px-4 py-2 rounded-md text-white font-medium focus:outline-none ${isLoading || !apiKey.trim()
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
                                            Testing...
                                        </span>
                                    ) : (
                                        'Save & Test Connection'
                                    )}
                                </button>
                            </div>
                        </form>
                    </div>
                </div>
            </div>
        </div>
    );
}