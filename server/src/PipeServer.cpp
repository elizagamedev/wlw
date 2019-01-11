#include "PipeServer.h"

#include "HookEvent.h"
#include <windows.h>

PipeServer::PipeServer(std::wstring pipe_name, OnConnect on_connect)
    : pipe_name_(std::move(pipe_name))
    , on_connect_(std::move(on_connect))
    , events_(1)
    , num_free_connections_(0)
{
    events_[0] = CreateEventW(nullptr, TRUE, FALSE, nullptr);
    poll_thread_ = std::thread([this]() {
            try {
                auto r = poll();
                if (r.has_error()) {
                    poll_thread_outcome_ = r.error();
                }
            } catch (...) {
                poll_thread_outcome_ = std::current_exception();
            }
        });
}

PipeServer::~PipeServer()
{
    // Shut down thread
    SetEvent(events_[0]);
    poll_thread_.join();
    CloseHandle(events_[0]);
    // Disconnnect all
    for (size_t i = 0; i < size(); ++i) {
        Connection &conn = connections_[i];
        (void)conn.disconnect();
        if (conn.pipe_ != NULL && conn.pipe_ != INVALID_HANDLE_VALUE) {
            CloseHandle(conn.pipe_);
        }
        HANDLE event = events_[i+1];
        if (event != NULL) {
            CloseHandle(event);
        }
    }
}

outcome::checked<void, WindowsError> PipeServer::poll()
{
    for (;;) {
        // First, ensure that we have at least one free connection
        if (num_free_connections_ == 0) {
            OUTCOME_TRY(grow(16));
        }
        // Now poll
        DWORD wait = WaitForMultipleObjects(
            static_cast<DWORD>(events_.size()), events_.data(), FALSE, INFINITE);
        if (wait == WAIT_FAILED) {
            return WindowsError::get_last();
        }
        if (wait == WAIT_TIMEOUT) {
            throw std::logic_error("pipe wait somehow timed out");
        }
        if (wait == WAIT_OBJECT_0) {
            break;
        }
        size_t index = wait - WAIT_OBJECT_0 - 1;
        Connection &conn = connections_[index];
        if (conn.io_outcome_.has_value()) {
            poll_thread_outcome_ = conn.io_outcome_.value().as_failure();
            break;
        }
        if (!conn.connection_active_) {
            DWORD cbRet;
            BOOL success = GetOverlappedResult(
                conn.pipe_,
                &conn.overlap_,
                &cbRet,
                FALSE);
            if (success) {
                conn.connection_active_ = true;
                --num_free_connections_;
                on_connect_(conn);
            } else {
                OUTCOME_TRY(conn.reconnect());
            }
        }
    }
    return outcome::success();
}

size_t PipeServer::size() const
{
    return connections_.size();
}

outcome::checked<void, WindowsError> PipeServer::grow(size_t amount)
{
    size_t new_size = events_.size() + amount;
    size_t old_size = events_.size();
    events_.resize(new_size + 1, NULL);
    connections_.resize(new_size);
    num_free_connections_ += amount;
    // Allocate new events and pipes
    for (size_t i = old_size; i < new_size; ++i) {
        events_[i+1] = CreateEventW(nullptr, TRUE, TRUE, nullptr);
        if (events_[i+1] == NULL) {
            return WindowsError::get_last();
        }
        connections_[i].cl_ = this;
        connections_[i].overlap_.hEvent = events_[i+1];
        connections_[i].pipe_ = CreateNamedPipeW(
            pipe_name_.c_str(),
            PIPE_ACCESS_DUPLEX |     // read/write access
            FILE_FLAG_OVERLAPPED,    // overlapped mode
            PIPE_TYPE_MESSAGE |      // message-type pipe
            PIPE_READMODE_MESSAGE |  // message-read mode
            PIPE_WAIT,               // blocking mode
            PIPE_UNLIMITED_INSTANCES,// number of instances
            sizeof(HookEvent),       // output buffer size
            sizeof(HookEvent),       // input buffer size
            0,                       // client time-out
            NULL);                   // default security attributes
        if (connections_[i].pipe_ == INVALID_HANDLE_VALUE) {
            return WindowsError::get_last();
        }
        OUTCOME_TRY(connections_[i].connect());
    }
    return outcome::success();
}


PipeServer::Connection::Connection()
    : cl_(nullptr)
    , pipe_(NULL)
    , connection_active_(false)
{
}

outcome::checked<void, WindowsError> PipeServer::Connection::disconnect()
{
    if (pipe_ != NULL && pipe_ != INVALID_HANDLE_VALUE && connection_active_) {
        connection_active_ = false;
        ++cl_->num_free_connections_;
        if (!DisconnectNamedPipe(pipe_)) {
            return WindowsError::get_last();
        }
    }
    return outcome::success();
}

outcome::checked<void, WindowsError> PipeServer::Connection::connect()
{
    if (pipe_ != NULL && pipe_ != INVALID_HANDLE_VALUE && !connection_active_) {
        ConnectNamedPipe(pipe_, &overlap_);
        DWORD last_error = GetLastError();
        switch (last_error) {
        case ERROR_IO_PENDING:
            connection_active_ = false;
            break;
        case ERROR_PIPE_CONNECTED:
            connection_active_ = true;
            --cl_->num_free_connections_;
            cl_->on_connect_(*this);
            break;
        default:
            return WindowsError(last_error);
        }
    }
    return outcome::success();
}

outcome::checked<void, WindowsError> PipeServer::Connection::reconnect()
{
    OUTCOME_TRY(disconnect());
    OUTCOME_TRY(connect());
    return outcome::success();
}

void PipeServer::Connection::write(const void *data, size_t len, OnCompletedIo on_completed_io, OnFailedIo on_failed_io)
{
    on_completed_io_ = std::move(on_completed_io);
    on_failed_io_ = std::move(on_failed_io);
    BOOL result = WriteFileEx(
        pipe_,
        data,
        static_cast<DWORD>(len),
        &overlap_,
        &on_completed_io_bootstrap);
    if (!result) {
        auto r = reconnect();
        if (r.has_error()) {
            io_outcome_ = r.error();
            SetEvent(overlap_.hEvent);
        }
        on_failed_io_(WindowsError::get_last());
    }
}

void PipeServer::Connection::read(void *data, size_t len, OnCompletedIo on_completed_io, OnFailedIo on_failed_io)
{
    on_completed_io_ = std::move(on_completed_io);
    on_failed_io_ = std::move(on_failed_io);
    BOOL result = ReadFileEx(
        pipe_,
        data,
        static_cast<DWORD>(len),
        &overlap_,
        &on_completed_io_bootstrap);
    if (!result) {
        auto r = reconnect();
        if (r.has_error()) {
            io_outcome_ = r.error();
            SetEvent(overlap_.hEvent);
        }
        on_failed_io_(WindowsError::get_last());
    }
}

void CALLBACK PipeServer::Connection::on_completed_io_bootstrap(
    DWORD dwErrorCode,
    DWORD dwNumberOfBytesTransferred,
    LPOVERLAPPED lpOverlapped)
{
    Connection *conn = reinterpret_cast<Connection*>(lpOverlapped);
    try {
        if (dwErrorCode) {
            auto r = conn->reconnect();
            if (r.has_error()) {
                conn->io_outcome_ = r.error();
                SetEvent(conn->overlap_.hEvent);
            }
            conn->on_failed_io_(WindowsError(dwErrorCode));
        } else {
            conn->on_completed_io_(*conn, dwNumberOfBytesTransferred);
        }
    } catch (...) {
        conn->io_outcome_ = std::current_exception();
        SetEvent(conn->overlap_.hEvent);
    }
}
