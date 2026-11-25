/* Diagnostic only: native Vulkan device creation outside Rust/wgpu/CubeCL. */
#include <stdio.h>
#include <stdlib.h>
#include <vulkan/vulkan.h>

int main(void) {
    VkInstance instance = VK_NULL_HANDLE;
    const VkApplicationInfo app = {
        .sType = VK_STRUCTURE_TYPE_APPLICATION_INFO,
        .pApplicationName = "pancake-sanitizer-boundary-probe",
        .apiVersion = VK_API_VERSION_1_3,
    };
    const VkInstanceCreateInfo info = {
        .sType = VK_STRUCTURE_TYPE_INSTANCE_CREATE_INFO,
        .pApplicationInfo = &app,
    };
    VkResult result = vkCreateInstance(&info, NULL, &instance);
    printf("vkCreateInstance=%d\n", result);
    if (result != VK_SUCCESS) return 1;
    uint32_t count = 0;
    result = vkEnumeratePhysicalDevices(instance, &count, NULL);
    if (result != VK_SUCCESS || count == 0) return 2;
    VkPhysicalDevice *devices = calloc(count, sizeof(*devices));
    if (devices == NULL) return 3;
    result = vkEnumeratePhysicalDevices(instance, &count, devices);
    if (result != VK_SUCCESS) return 4;
    int code = 5;
    for (uint32_t i = 0; i < count; ++i) {
        VkPhysicalDeviceProperties properties;
        vkGetPhysicalDeviceProperties(devices[i], &properties);
        printf("adapter=%s vendor=0x%x\n", properties.deviceName, properties.vendorID);
        if (properties.vendorID != 0x10de) continue;
        uint32_t families = 0;
        vkGetPhysicalDeviceQueueFamilyProperties(devices[i], &families, NULL);
        VkQueueFamilyProperties *queues = calloc(families, sizeof(*queues));
        if (queues == NULL) return 6;
        vkGetPhysicalDeviceQueueFamilyProperties(devices[i], &families, queues);
        for (uint32_t q = 0; q < families; ++q) {
            if (!(queues[q].queueFlags & VK_QUEUE_COMPUTE_BIT)) continue;
            float priority = 1.0f;
            const VkDeviceQueueCreateInfo queue = {
                .sType = VK_STRUCTURE_TYPE_DEVICE_QUEUE_CREATE_INFO,
                .queueFamilyIndex = q, .queueCount = 1, .pQueuePriorities = &priority,
            };
            const VkDeviceCreateInfo device_info = {
                .sType = VK_STRUCTURE_TYPE_DEVICE_CREATE_INFO,
                .queueCreateInfoCount = 1, .pQueueCreateInfos = &queue,
            };
            VkDevice device = VK_NULL_HANDLE;
            result = vkCreateDevice(devices[i], &device_info, NULL, &device);
            printf("vkCreateDevice=%d\n", result);
            if (result == VK_SUCCESS) vkDestroyDevice(device, NULL);
            code = result == VK_SUCCESS ? 0 : 7;
            break;
        }
        free(queues);
        break;
    }
    free(devices);
    vkDestroyInstance(instance, NULL);
    return code;
}
