.PHONY: docker-local docker-scan clean

IMAGE_NAME := socket-metrics-exporter

clean:
	rm -rf target reports

reports:
	mkdir -p reports

docker-local:
	docker build --ssh default -t $(IMAGE_NAME):latest .
	

docker-scan: docker-local reports
	docker run -it -v $${HOME}/.cache/trivy-docker:/cache -v $${PWD}/reports:/reports -v /var/run/docker.sock:/var/run/docker.sock --rm aquasec/trivy@sha256:3d1f862cb6c4fe13c1506f96f816096030d8d5ccdb2380a3069f7bf07daa86aa --cache-dir /cache image --format json --output=reports/trivy-$$(date +%Y%m%d%H%M%S).json $(IMAGE_NAME):latest
