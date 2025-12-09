'use client';

import React from 'react';
import Link from 'next/link';
import { usePathname } from 'next/navigation';

export default function Header() {
    const pathname = usePathname();

    return (
        <header className="bg-gray-800 text-white">
            <div className="container mx-auto px-4">
                <div className="flex items-center justify-between h-16">
                    <div className="flex items-center">
                        <Link href="/" className="font-bold text-xl">
                            Phoenix ORCH
                        </Link>
                    </div>

                    <nav className="flex space-x-4">
                        <Link
                            href="/dashboard"
                            className={`px-3 py-2 rounded-md text-sm font-medium ${pathname === '/dashboard'
                                    ? 'bg-gray-900 text-white'
                                    : 'text-gray-300 hover:bg-gray-700 hover:text-white'
                                }`}
                        >
                            Dashboard
                        </Link>

                        <Link
                            href="/settings"
                            className={`px-3 py-2 rounded-md text-sm font-medium ${pathname === '/settings'
                                    ? 'bg-gray-900 text-white'
                                    : 'text-gray-300 hover:bg-gray-700 hover:text-white'
                                }`}
                        >
                            Settings
                        </Link>
                    </nav>
                </div>
            </div>
        </header>
    );
}