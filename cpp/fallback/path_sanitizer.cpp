// FILE: ./cpp/fallback/path_sanitizer.cpp

#include <string>
#include <vector>

// Fallback C++ implementation of path normalization for Code-Command.
// Enforces CC-PATH by:
//   - Replacing backslashes with forward slashes
//   - Collapsing duplicate slashes
//   - Resolving "." and ".." segments. [file:2]
std::string sanitize_path(const std::string& input) {
    // Step 1: replace '\' with '/' and collapse duplicate slashes. [file:2]
    std::string tmp;
    tmp.reserve(input.size());
    bool lastWasSlash = false;

    for (char ch : input) {
        char c = ch;
        if (c == '\\') {
            c = '/';
        }
        if (c == '/') {
            if (!lastWasSlash) {
                tmp.push_back('/');
                lastWasSlash = true;
            }
        } else {
            tmp.push_back(c);
            lastWasSlash = false;
        }
    }

    // Step 2: split into segments and resolve "." and "..". [file:2]
    std::vector<std::string> segments;
    std::string current;

    for (char c : tmp) {
        if (c == '/') {
            if (!current.empty()) {
                segments.push_back(current);
                current.clear();
            }
        } else {
            current.push_back(c);
        }
    }
    if (!current.empty()) {
        segments.push_back(current);
    }

    std::vector<std::string> stack;
    for (const auto& seg : segments) {
        if (seg == "." || seg.empty()) {
            continue;
        }
        if (seg == "..") {
            if (!stack.empty()) {
                stack.pop_back();
            }
        } else {
            stack.push_back(seg);
        }
    }

    // Step 3: re-join with single '/'. [file:2]
    std::string result;
    for (std::size_t i = 0; i < stack.size(); ++i) {
        if (i > 0) {
            result.push_back('/');
        }
        result.append(stack[i]);
    }

    return result;
}
