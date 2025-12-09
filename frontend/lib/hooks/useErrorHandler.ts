'use client';

import { useState, useCallback } from 'react';

interface ErrorState {
    message: string;
    code?: string;
    isVisible: boolean;
}

export default function useErrorHandler() {
    const [error, setError] = useState<ErrorState>({
        message: '',
        code: undefined,
        isVisible: false,
    });

    const handleError = useCallback((err: unknown) => {
        // Process different error types
        if (err instanceof Error) {
            setError({
                message: err.message,
                code: 'ERROR',
                isVisible: true,
            });
        } else if (typeof err === 'string') {
            setError({
                message: err,
                code: 'ERROR',
                isVisible: true,
            });
        } else {
            setError({
                message: 'An unknown error occurred',
                code: 'UNKNOWN_ERROR',
                isVisible: true,
            });
        }

        // Log for debugging
        console.error('Error occurred:', err);
    }, []);

    const clearError = useCallback(() => {
        setError({
            message: '',
            code: undefined,
            isVisible: false,
        });
    }, []);

    const apiErrorHandler = useCallback(async <T>(
        promise: Promise<T>,
        customErrorMessage?: string
    ): Promise<T | null> => {
        try {
            clearError();
            return await promise;
        } catch (err) {
            if (customErrorMessage) {
                setError({
                    message: customErrorMessage,
                    code: 'API_ERROR',
                    isVisible: true,
                });
            } else {
                handleError(err);
            }
            return null;
        }
    }, [handleError, clearError]);

    return {
        error,
        handleError,
        clearError,
        apiErrorHandler,
    };
}