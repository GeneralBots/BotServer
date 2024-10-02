import cv from '@u4/opencv4nodejs'
import QRCodeReader from 'qrcode-reader';

class QRCodeProcessor {
    async decodeQRCode(imagePath) {
        const image = cv.imread(imagePath, cv.IMREAD_COLOR);
        const grayImage = image.bgrToGray();
        const blurredImage = grayImage.gaussianBlur(new cv.Size(5, 5), 0);
        const edges = blurredImage.canny(50, 150);

        const contour = this.findQRCodeContour(edges);
        if (!contour) {
            throw new Error('QR Code não encontrado.');
        }

        const transformedImage = this.getPerspectiveTransform(image, contour);
        return await this.readQRCode(transformedImage);
    }

    findQRCodeContour(edges) {
        const contours = edges.findContours(cv.RETR_EXTERNAL, cv.CHAIN_APPROX_SIMPLE);
        let maxContour = null;
        let maxArea = 0;

        contours.forEach(contour => {
            const area = contour.area;
            if (area > maxArea) {
                maxArea = area;
                maxContour = contour;
            }
        });

        return maxContour;
    }

    getPerspectiveTransform(image, contour) {
        // Ensure the contour has at least 4 points
        const points = contour.getPoints();
        if (points.length < 4) {
            throw new Error("Contour must have at least 4 points.");
        }

        // Get the first four points
        const srcPoints = points.slice(0, 4).map(point => new cv.Point2(point.x, point.y));

        // Define destination points for the perspective transform
        const dst = [
            new cv.Point2(0, 0),
            new cv.Point2(300, 0),
            new cv.Point2(300, 300),
            new cv.Point2(0, 300)
        ];

        // Get the perspective transform matrix
        const M = cv.getPerspectiveTransform(srcPoints, dst);

        // Create a new Mat for the transformed image
        const transformedImage = new cv.Mat(300, 300, cv.CV_8UC3);

        // Manually apply the perspective transformation
        for (let y = 0; y < transformedImage.rows; y++) {
            for (let x = 0; x < transformedImage.cols; x++) {
                const srcPoint = this.applyPerspective(M, x, y);
                const srcX = Math.round(srcPoint.x);
                const srcY = Math.round(srcPoint.y);

                // Check if the mapped source point is within the bounds of the source image
                if (srcX >= 0 && srcX < image.cols && srcY >= 0 && srcY < image.rows) {
                    const pixelValue = image.atVector(srcY, srcX); // Use atVector to get pixel values
                    transformedImage.set(y, x, pixelValue);
                }
            }
        }

        return transformedImage;
    }

    applyPerspective(M, x, y) {
        const a = M.getData(); // Get the matrix data
        const denominator = a[6] * x + a[7] * y + 1; // Calculate the denominator
        const newX = (a[0] * x + a[1] * y + a[2]) / denominator;
        const newY = (a[3] * x + a[4] * y + a[5]) / denominator;
        return new cv.Point2(newX, newY);
    }

    async readQRCode(image) {
        return new Promise((resolve, reject) => {
            const qrCodeReader = new QRCodeReader();
            qrCodeReader.decode(image.getData(), (err, result) => {
                if (err) {
                    return reject(err);
                }
                resolve(result.result);
            });
        });
    }
}

export default QRCodeProcessor;
