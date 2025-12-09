'use client';

import React, { useState, useEffect, useRef } from 'react';
import ChatMessage from '../ui/ChatMessage';
import ChatInput from '../ui/ChatInput';
import { Message } from '@/types';
import { executeCommand } from '@/lib/api/orchService';
import { v4 as uuidv4 } from 'uuid';

export default function ChatContainer() {
    const [messages, setMessages] = useState<Message[]>([]);
    const [isLoading, setIsLoading] = useState<boolean>(false);
    const [error, setError] = useState<string | null>(null);
    const [expandedMessageId, setExpandedMessageId] = useState<string | null>(null);

    const messagesEndRef = useRef<HTMLDivElement>(null);

    // Scroll to bottom on new messages
    useEffect(() => {
        messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
    }, [messages]);

    const handleSendMessage = async (content: string) => {
        if (!content.trim()) return;

        // Create the user message
        const userMessage: Message = {
            id: uuidv4(),
            content,
            isUser: true,
            timestamp: new Date(),
        };

        setMessages(prev => [...prev, userMessage]);
        setIsLoading(true);
        setError(null);

        try {
            // Send to API
            const response = await executeCommand(content);

            // Create the response message
            const responseMessage: Message = {
                id: uuidv4(),
                content: response.result,
                isUser: false,
                timestamp: new Date(),
                response,
            };

            setMessages(prev => [...prev, responseMessage]);
        } catch (err) {
            setError(
                err instanceof Error
                    ? err.message
                    : 'An error occurred while processing your request.'
            );
        } finally {
            setIsLoading(false);
        }
    };

    const toggleMessageExpand = (messageId: string) => {
        setExpandedMessageId(prev => prev === messageId ? null : messageId);
    };

    return (
        <div className="flex flex-col h-[calc(100vh-64px)]">
            <div className="flex-1 overflow-y-auto p-4">
                {messages.length === 0 ? (
                    <div className="flex items-center justify-center h-full text-gray-500">
                        <div className="text-center">
                            <h2 className="text-2xl font-semibold mb-2">Welcome to Phoenix ORCH</h2>
                            <p>Start a conversation by typing a message below</p>
                        </div>
                    </div>
                ) : (
                    <div className="space-y-4">
                        {messages.map(message => (
                            <ChatMessage
                                key={message.id}
                                message={message}
                                expanded={expandedMessageId === message.id}
                                onToggleExpand={() => toggleMessageExpand(message.id)}
                            />
                        ))}
                    </div>
                )}

                {error && (
                    <div className="bg-red-50 text-red-700 p-3 rounded-md mb-4">
                        <div className="font-medium">Error</div>
                        <div className="text-sm">{error}</div>
                    </div>
                )}

                <div ref={messagesEndRef} />
            </div>

            <ChatInput onSendMessage={handleSendMessage} isLoading={isLoading} />
        </div>
    );
}