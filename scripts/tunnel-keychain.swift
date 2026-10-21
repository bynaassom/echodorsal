import Foundation
import Security

let service = "echodorsal.cloudflared"
let tunnelAccount = "tunnel-credentials"
let apiTokenAccount = "cloudflare-api-token"
let pendingAccount = "tunnel-pending"

func printUsageAndExit(code: Int32 = 1) -> Never {
    let usage = "usage: tunnel-keychain <put|put-new|get|delete|put-api-token|get-api-token|delete-api-token|put-pending|get-pending|delete-pending>\n"
    if let data = usage.data(using: .utf8) {
        try? FileHandle.standardError.write(contentsOf: data)
    }
    exit(code)
}

func failWithCode(_ message: String, code: Int32 = 1) -> Never {
    let msg = "\(message)\n"
    if let data = msg.data(using: .utf8) {
        try? FileHandle.standardError.write(contentsOf: data)
    }
    exit(code)
}

func readStandardInput() -> Data {
    let inputData: Data
    do {
        guard let data = try FileHandle.standardInput.readToEnd() else {
            failWithCode("error: empty input", code: 1)
        }
        inputData = data
    } catch {
        failWithCode("error: failed to read standard input", code: 1)
    }

    if inputData.isEmpty || inputData.allSatisfy({ $0 == 0x20 || $0 == 0x09 || $0 == 0x0A || $0 == 0x0D }) {
        failWithCode("error: empty input", code: 1)
    }
    return inputData
}

func putItem(service: String, account: String) -> Never {
    let inputData = readStandardInput()

    let query: [String: Any] = [
        kSecClass as String: kSecClassGenericPassword,
        kSecAttrService as String: service,
        kSecAttrAccount as String: account,
        kSecValueData as String: inputData,
        kSecAttrAccessible as String: kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly
    ]

    let addStatus = SecItemAdd(query as CFDictionary, nil)
    if addStatus == errSecDuplicateItem {
        let matchQuery: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecAttrAccount as String: account
        ]
        let updateAttrs: [String: Any] = [
            kSecValueData as String: inputData,
            kSecAttrAccessible as String: kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly
        ]
        let updateStatus = SecItemUpdate(matchQuery as CFDictionary, updateAttrs as CFDictionary)
        if updateStatus != errSecSuccess {
            failWithCode("error: failed to update keychain item (\(updateStatus))", code: 1)
        }
    } else if addStatus != errSecSuccess {
        failWithCode("error: failed to add keychain item (\(addStatus))", code: 1)
    }

    exit(0)
}

func putItemAddOnly(service: String, account: String) -> Never {
    let inputData = readStandardInput()

    let query: [String: Any] = [
        kSecClass as String: kSecClassGenericPassword,
        kSecAttrService as String: service,
        kSecAttrAccount as String: account,
        kSecValueData as String: inputData,
        kSecAttrAccessible as String: kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly
    ]

    let addStatus = SecItemAdd(query as CFDictionary, nil)
    if addStatus == errSecSuccess {
        exit(0)
    } else if addStatus == errSecDuplicateItem {
        failWithCode("error: duplicate item", code: 3)
    } else {
        failWithCode("error: failed to add keychain item (\(addStatus))", code: 1)
    }
}

func getItem(service: String, account: String) -> Never {
    let query: [String: Any] = [
        kSecClass as String: kSecClassGenericPassword,
        kSecAttrService as String: service,
        kSecAttrAccount as String: account,
        kSecReturnData as String: kCFBooleanTrue as Any,
        kSecMatchLimit as String: kSecMatchLimitOne
    ]

    var result: CFTypeRef?
    let getStatus = SecItemCopyMatching(query as CFDictionary, &result)
    if getStatus == errSecItemNotFound {
        failWithCode("error: item not found", code: 2)
    } else if getStatus != errSecSuccess {
        failWithCode("error: failed to retrieve keychain item (\(getStatus))", code: 1)
    }

    guard let data = result as? Data, !data.isEmpty else {
        failWithCode("error: empty keychain item data", code: 1)
    }

    do {
        try FileHandle.standardOutput.write(contentsOf: data)
    } catch {
        failWithCode("error: failed to write standard output", code: 1)
    }
    exit(0)
}

func deleteItem(service: String, account: String) -> Never {
    let query: [String: Any] = [
        kSecClass as String: kSecClassGenericPassword,
        kSecAttrService as String: service,
        kSecAttrAccount as String: account
    ]

    let deleteStatus = SecItemDelete(query as CFDictionary)
    if deleteStatus == errSecSuccess {
        exit(0)
    } else if deleteStatus == errSecItemNotFound {
        failWithCode("error: item not found", code: 2)
    } else {
        failWithCode("error: failed to delete keychain item (\(deleteStatus))", code: 1)
    }
}

let args = CommandLine.arguments
guard args.count == 2 else {
    printUsageAndExit()
}

let command = args[1]

switch command {
case "put":
    putItem(service: service, account: tunnelAccount)
case "put-new":
    putItemAddOnly(service: service, account: tunnelAccount)
case "get":
    getItem(service: service, account: tunnelAccount)
case "delete":
    deleteItem(service: service, account: tunnelAccount)
case "put-api-token":
    putItem(service: service, account: apiTokenAccount)
case "get-api-token":
    getItem(service: service, account: apiTokenAccount)
case "delete-api-token":
    deleteItem(service: service, account: apiTokenAccount)
case "put-pending":
    putItemAddOnly(service: service, account: pendingAccount)
case "get-pending":
    getItem(service: service, account: pendingAccount)
case "delete-pending":
    deleteItem(service: service, account: pendingAccount)
default:
    printUsageAndExit()
}
