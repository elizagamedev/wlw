#pragma once

#include <windows.h>
#include <vector>
#include <string>
#include <functional>
#include <thread>
#include <outcome.hpp>
#include <optional>
#include "WindowsError.h"

namespace outcome = OUTCOME_V2_NAMESPACE;


class PipeServer
{
public:
    class Connection {
        friend class PipeServer;
    public:
        typedef std::function<void(Connection&, size_t)> OnCompletedIo;
        typedef std::function<void(WindowsError)> OnFailedIo;

        Connection();

        outcome::checked<void, WindowsError> reconnect();
        void write(const void *data, size_t len, OnCompletedIo on_completed_io, OnFailedIo on_failed_io = [](WindowsError){});
        void read(void *data, size_t len, OnCompletedIo on_completed_io, OnFailedIo on_failed_io = [](WindowsError){});

    private:
        outcome::checked<void, WindowsError> connect();
        outcome::checked<void, WindowsError> disconnect();

        static void CALLBACK on_completed_io_bootstrap(DWORD dwErrorCode,
                                                       DWORD dwNumberOfBytesTransferred,
                                                       LPOVERLAPPED lpOverlapped);

        // This must be the first member of the class
        OVERLAPPED overlap_;
        PipeServer *cl_;
        HANDLE pipe_;
        bool connection_active_;
        OnCompletedIo on_completed_io_;
        OnFailedIo on_failed_io_;
        std::optional<outcome::outcome<void, WindowsError>> io_outcome_;
    };

    typedef std::function<void(Connection&)> OnConnect;
    PipeServer(std::wstring pipe_name, OnConnect on_connect);
    ~PipeServer();

private:
    outcome::checked<void, WindowsError> poll();
    size_t size() const;
    outcome::checked<void, WindowsError> grow(size_t amount);

    std::wstring pipe_name_;
    OnConnect on_connect_;
    std::vector<HANDLE> events_;
    std::vector<Connection> connections_;
    size_t num_free_connections_;

    std::thread poll_thread_;
    std::optional<outcome::outcome<void, WindowsError>> poll_thread_outcome_;
};
